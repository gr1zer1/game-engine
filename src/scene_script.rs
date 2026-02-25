use std::collections::VecDeque;

use crate::{
    achievements::AchievementManager, audio::AudioEngine, dialogue_ui::DialogueUi,
    game_object::SceneObject, tex::Tex,
};

// Signals are broadcast by the app (input/system events) to all active scripts.
#[derive(Clone, Copy, Debug)]
pub enum ScriptSignal {
    SkipWait,
}

// Per-frame services exposed to scripts.
pub struct ScriptContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub tex: &'a mut Tex,
    pub dialogue_ui: &'a mut DialogueUi,
    pub achievements: &'a mut AchievementManager,
    #[allow(dead_code)]
    pub audio: Option<&'a mut AudioEngine>,
}

// Unity-style lifecycle: start once, then update every frame.
pub trait SceneScript {
    fn start(&mut self, _context: &mut ScriptContext<'_>) -> Result<(), String> {
        Ok(())
    }

    fn update(&mut self, dt: f32, context: &mut ScriptContext<'_>) -> Result<(), String>;

    fn on_signal(&mut self, _signal: ScriptSignal) {}

    fn is_finished(&self) -> bool {
        false
    }
}

struct ScriptEntry {
    script: Box<dyn SceneScript>,
    started: bool,
}

pub struct SceneRunner {
    scripts: Vec<ScriptEntry>,
}

impl SceneRunner {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
        }
    }

    pub fn with_scripts(scripts: Vec<Box<dyn SceneScript>>) -> Self {
        let mut runner = Self::new();
        for script in scripts {
            runner.add_script(script);
        }
        runner
    }

    pub fn add_script(&mut self, script: Box<dyn SceneScript>) {
        self.scripts.push(ScriptEntry {
            script,
            started: false,
        });
    }

    pub fn send_signal(&mut self, signal: ScriptSignal) {
        for entry in &mut self.scripts {
            entry.script.on_signal(signal);
        }
    }

    pub fn is_finished(&self) -> bool {
        self.scripts.iter().all(|entry| entry.script.is_finished())
    }

    pub fn update(&mut self, dt: f32, context: &mut ScriptContext<'_>) -> Result<(), String> {
        for entry in &mut self.scripts {
            // Skip scripts that already reached terminal state.
            if entry.script.is_finished() {
                continue;
            }

            // start() is called exactly once before first update().
            if !entry.started {
                entry.script.start(context)?;
                entry.started = true;
            }

            if entry.script.is_finished() {
                continue;
            }

            entry.script.update(dt, context)?;
        }

        Ok(())
    }
}

// Simple timeline command language for cutscene-like scripting.
#[derive(Clone, Debug)]
pub enum SceneCommand {
    Spawn(SceneObject),
    Apply(SceneObject),
    Wait(f32),
}

pub fn spawn(object: impl Into<SceneObject>) -> SceneCommand {
    SceneCommand::Spawn(object.into())
}

pub fn apply(object: impl Into<SceneObject>) -> SceneCommand {
    SceneCommand::Apply(object.into())
}

pub fn wait(seconds: f32) -> SceneCommand {
    SceneCommand::Wait(seconds.max(0.0))
}

pub struct TimelineScript {
    pending: VecDeque<SceneCommand>,
    wait_remaining: f32,
}

impl TimelineScript {
    pub fn new(commands: Vec<SceneCommand>) -> Self {
        Self {
            pending: commands.into(),
            wait_remaining: 0.0,
        }
    }

    fn process_commands(
        &mut self,
        mut dt: f32,
        context: &mut ScriptContext<'_>,
    ) -> Result<(), String> {
        loop {
            // Consume frame time against pending wait, if any.
            if self.wait_remaining > 0.0 {
                if dt <= 0.0 {
                    break;
                }

                if dt >= self.wait_remaining {
                    dt -= self.wait_remaining;
                    self.wait_remaining = 0.0;
                } else {
                    self.wait_remaining -= dt;
                    break;
                }
            }

            let Some(command) = self.pending.pop_front() else {
                break;
            };

            match command {
                SceneCommand::Wait(seconds) => {
                    // Pause command processing until this timer reaches zero.
                    self.wait_remaining = seconds.max(0.0);
                }
                SceneCommand::Spawn(object) | SceneCommand::Apply(object) => {
                    Self::apply_object(object, context)?;
                }
            }
        }

        Ok(())
    }

    fn apply_object(object: SceneObject, context: &mut ScriptContext<'_>) -> Result<(), String> {
        match object {
            // Sprite definitions are applied to the texture renderer.
            SceneObject::Sprite(sprite) => {
                context
                    .tex
                    .apply_game_object_from_definition(context.device, context.queue, sprite)
            }
            // Dialogue objects are routed to the dialogue UI system.
            SceneObject::Dialogue(dialogue) => {
                context.dialogue_ui.apply_dialogue_object(dialogue);
                Ok(())
            }
        }
    }
}

impl SceneScript for TimelineScript {
    fn start(&mut self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        self.process_commands(0.0, context)
    }

    fn update(&mut self, dt: f32, context: &mut ScriptContext<'_>) -> Result<(), String> {
        self.process_commands(dt, context)
    }

    fn on_signal(&mut self, signal: ScriptSignal) {
        if matches!(signal, ScriptSignal::SkipWait) {
            self.wait_remaining = 0.0;
        }
    }

    fn is_finished(&self) -> bool {
        self.pending.is_empty() && self.wait_remaining <= 0.0
    }
}
