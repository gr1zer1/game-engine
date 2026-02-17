use crate::game_object::{GameObject2D, RenderLayer};

pub fn read_initial_scene_objects() -> Vec<GameObject2D> {
    vec![
        GameObject2D::new(
            [0.0, 0.0],
            [2.0, 2.0],
            "src/happy_tree.png",
            RenderLayer::Background,
            1,
        ),
        GameObject2D::new(
            [0.0, -0.5],
            [1.0, 1.0],
            "src/happy_tree.png",
            RenderLayer::Ui,
            10,
        ),
        GameObject2D::new(
            [1.2, 0.0],
            [0.8, 0.8],
            "src/happy_tree.png",
            RenderLayer::Character,
            5,
        )
        .with_hidden(true),
    ]
}
