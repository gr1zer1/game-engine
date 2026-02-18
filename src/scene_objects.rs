use crate::{
    game_object::{DialogueBoxObject, GameObject2D, RenderLayer},
    scene_script::{SceneCommand, apply, spawn, wait},
};

pub fn read_initial_scene_script() -> Vec<SceneCommand> {
    let game_object = GameObject2D::new(
        [0.0, 0.0],
        [1.0, 1.0],
        "src/image.jpg",
        RenderLayer::Character,
        5,
    )
    .with_hidden(false);

    vec![
        spawn(GameObject2D::new(
            [0.0, 0.0],
            [2.0, 2.0],
            "src/happy_tree.png",
            RenderLayer::Background,
            1,
        )),
        spawn(GameObject2D::new(
            [0.0, -0.5],
            [1.0, 1.0],
            "src/happy_tree.png",
            RenderLayer::Ui,
            10,
        )),
        spawn(GameObject2D::new(
            [1.2, 0.0],
            [0.8, 0.8],
            "src/happy_tree.png",
            RenderLayer::Character,
            5,
        )),
        spawn(game_object.clone()),
        spawn(DialogueBoxObject::new("But there are only two baskets.")),
        wait(5.0),
        apply(game_object.with_hidden(true)),
    ]
}
