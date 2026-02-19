use crate::{
    game_object::DialogueBoxObject,
    scene_script::{SceneScript, ScriptContext, ScriptSignal},
};

// Minimal gameplay script: show intro dialogue, then close it on SkipWait.
pub struct Game {
    dialogue: DialogueBoxObject,
    visible: bool,
    close_requested: bool,
    finished: bool,
}

impl Game {
    pub fn new() -> Self {
        Self {
            dialogue: DialogueBoxObject::new("Hello my name Danil Ardashew ", "Danil Adrdashew")
                .with_id("intro_dialogue"),
            visible: true,
            close_requested: false,
            finished: false,
        }
    }

    fn apply_current_state(&self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        let object = self.dialogue.clone().with_hidden(!self.visible);
        context.dialogue_ui.apply_dialogue_object(object);
        Ok(())
    }
}

impl SceneScript for Game {
    fn start(&mut self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        self.apply_current_state(context)
    }

    fn update(&mut self, _dt: f32, context: &mut ScriptContext<'_>) -> Result<(), String> {
        if self.close_requested && self.visible {
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
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}
