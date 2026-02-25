use crate::{
    game_object::{DialogueBoxObject, GameObject2D, RenderLayer},
    scene_script::{SceneCommand, SceneScript, TimelineScript, apply, spawn, wait},
    scripts::{BlinkSpriteScript, BobSpriteScript, Game},
};

fn blinking_sprite() -> GameObject2D {
    GameObject2D::new(
        [1.5, -0.8],
        [0.5, 0.5],
        "src/happy_tree.png",
        RenderLayer::Ui,
        20,
    )
    .with_id("blink_sprite")
}

fn bobbing_sprite() -> GameObject2D {
    GameObject2D::new(
        [-1.3, -0.15],
        [0.75, 0.75],
        "src/happy_tree.png",
        RenderLayer::Character,
        7,
    )
    .with_id("bob_sprite")
}

fn read_initial_scene_commands() -> Vec<SceneCommand> {
    // Timeline commands are currently optional because behavior is script-driven.
    let game_object = GameObject2D::new(
        [0.0, 0.0],
        [1.0, 1.0],
        "src/image.jpg",
        RenderLayer::Character,
        5,
    )
    .with_hidden(false);

    vec![]
}

pub fn create_initial_scene_scripts() -> Vec<Box<dyn SceneScript>> {
    // Register all scripts that should be active at scene startup.
    vec![
        Box::new(TimelineScript::new(read_initial_scene_commands())),
        Box::new(BlinkSpriteScript::new(blinking_sprite(), 0.45)),
        Box::new(BobSpriteScript::new(bobbing_sprite(), 0.18, 2.8)),
        Box::new(Game::new(GameObject2D::new(
            [0.0, 0.0],
            [1.0, 1.0],
            "src/image.jpg",
            RenderLayer::Character,
            5,
        ))),
    ]
}
