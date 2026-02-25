use crate::{
    game_object::{DialogueBoxObject, GameObject2D},
    scene_script::{SceneScript, ScriptContext, ScriptSignal},
    scripts::achievements as achievement_scripts,
};

// Minimal gameplay script: show intro dialogue, then close it on SkipWait.
pub struct Game {
    dialogue: DialogueBoxObject,
    image: GameObject2D,
    visible: bool,
    close_requested: bool,
    skip_signal_received: bool,
    finished: bool,
}

impl Game {
    pub fn new(image: GameObject2D) -> Self {
        Self {
            dialogue: DialogueBoxObject::new("Hello my name Ajzakun.", "Ajzakun")
                .with_id("intro_dialogue"),
            image,
            visible: true,
            close_requested: false,
            skip_signal_received: false,
            finished: false,
        }
    }

    fn apply_current_state(&self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        let object = self.dialogue.clone().with_hidden(!self.visible);
        let image_obj = self.image.clone().with_hidden(!self.visible);
        context.dialogue_ui.apply_dialogue_object(object);
        #[allow(unused)]
        context
            .tex
            .apply_game_object_from_definition(&context.device, &context.queue, image_obj);
        Ok(())
    }
}

impl SceneScript for Game {
    fn start(&mut self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        achievement_scripts::trigger(context.achievements, "game_started");
        self.apply_current_state(context)
    }

    fn update(&mut self, _dt: f32, context: &mut ScriptContext<'_>) -> Result<(), String> {
        if self.close_requested && self.visible {
            achievement_scripts::trigger(context.achievements, "intro_closed");
            if self.skip_signal_received {
                achievement_scripts::trigger(context.achievements, "intro_skipped");
            }
            if !achievement_scripts::is_unlocked(context.achievements, "script_reward") {
                achievement_scripts::grant(context.achievements, "script_reward");
            }

            self.visible = false;
            self.finished = true;
            self.apply_current_state(context)?;
        }

        Ok(())
    }

    fn on_signal(&mut self, signal: ScriptSignal) {
        // SkipWait is triggered by Enter/Space in the input action map.
        if matches!(signal, ScriptSignal::SkipWait) {
            self.close_requested = true;
            self.skip_signal_received = true;
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}
