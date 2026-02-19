use crate::{
    game_object::GameObject2D,
    scene_script::{SceneScript, ScriptContext},
};

// Applies a vertical sine-wave motion to a sprite.
pub struct BobSpriteScript {
    sprite: GameObject2D,
    base_y: f32,
    amplitude: f32,
    speed: f32,
    elapsed: f32,
}

impl BobSpriteScript {
    pub fn new(sprite: GameObject2D, amplitude: f32, speed: f32) -> Self {
        Self {
            base_y: sprite.position.y,
            sprite,
            amplitude: amplitude.abs(),
            speed,
            elapsed: 0.0,
        }
    }
}

impl SceneScript for BobSpriteScript {
    fn start(&mut self, context: &mut ScriptContext<'_>) -> Result<(), String> {
        context.tex.apply_game_object_from_definition(
            context.device,
            context.queue,
            self.sprite.clone(),
        )
    }

    fn update(&mut self, dt: f32, context: &mut ScriptContext<'_>) -> Result<(), String> {
        self.elapsed += dt.max(0.0);

        let mut object = self.sprite.clone();
        // base_y + sin(t) gives smooth floating motion.
        object.position.y = self.base_y + self.amplitude * (self.elapsed * self.speed).sin();

        context
            .tex
            .apply_game_object_from_definition(context.device, context.queue, object)
    }
}
