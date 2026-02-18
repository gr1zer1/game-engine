use std::collections::VecDeque;

use crate::{dialogue_ui::DialogueUi, game_object::SceneObject, tex::Tex};

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

pub struct SceneTimeline {
    pending: VecDeque<SceneCommand>,
    wait_remaining: f32,
}

impl SceneTimeline {
    pub fn new(commands: Vec<SceneCommand>) -> Self {
        Self {
            pending: commands.into(),
            wait_remaining: 0.0,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.pending.is_empty() && self.wait_remaining <= 0.0
    }

    pub fn skip_wait(&mut self) {
        self.wait_remaining = 0.0;
    }

    pub fn update(
        &mut self,
        mut dt: f32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tex: &mut Tex,
        dialogue_ui: &mut DialogueUi,
    ) -> Result<(), String> {
        loop {
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
                    self.wait_remaining = seconds.max(0.0);
                }
                SceneCommand::Spawn(object) | SceneCommand::Apply(object) => {
                    Self::apply_object(object, device, queue, tex, dialogue_ui)?;
                }
            }
        }

        Ok(())
    }

    fn apply_object(
        object: SceneObject,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tex: &mut Tex,
        dialogue_ui: &mut DialogueUi,
    ) -> Result<(), String> {
        match object {
            SceneObject::Sprite(sprite) => {
                tex.apply_game_object_from_definition(device, queue, sprite)
            }
            SceneObject::Dialogue(dialogue) => {
                dialogue_ui.apply_dialogue_object(dialogue);
                Ok(())
            }
        }
    }
}
