use crate::{
    game_object::GameObject2D,
    scene_script::{SceneScript, ScriptContext},
};

// Toggles sprite visibility at a fixed interval.
pub struct BlinkSpriteScript {
    sprite: GameObject2D,
    interval: f32,
    elapsed: f32,
    visible: bool,
}

impl BlinkSpriteScript {
    pub fn new(sprite: GameObject2D, interval: f32) -> Self {
        Self {
            visible: !sprite.hidden,
            sprite,
            interval: interval.max(0.01),
            elapsed: 0.0,
        }
    }

    fn apply_current_state(&self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        let object = self.sprite.clone().with_hidden(!self.visible);
        context
            .tex
            .apply_game_object_from_definition(context.device, context.queue, object)
    }
}

impl SceneScript for BlinkSpriteScript {
    fn start(&mut self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        self.apply_current_state(context)
    }

    fn update(&mut self, dt: f32, context: &mut ScriptContext<'_>) -> Result<(), String> {
        self.elapsed += dt.max(0.0);

        // Handle large frame times by consuming multiple intervals.
        let mut toggles = 0_u32;
        while self.elapsed >= self.interval {
            self.elapsed -= self.interval;
            toggles = toggles.saturating_add(1);
        }

        if toggles % 2 == 1 {
            self.visible = !self.visible;
            self.apply_current_state(context)?;
        }

        Ok(())
    }
}
