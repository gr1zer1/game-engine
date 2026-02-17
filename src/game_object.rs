use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Background,
    Character,
    Ui,
}

impl RenderLayer {
    pub const fn order(self) -> i32 {
        match self {
            Self::Background => 0,
            Self::Character => 1,
            Self::Ui => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameObject2D {
    pub position: Vec2,
    pub scale: Vec2,
    pub texture_path: String,
    pub layer: RenderLayer,
    pub z_index: i32,
    pub hidden: bool,
}

impl GameObject2D {
    pub fn new(
        position: [f32; 2],
        scale: [f32; 2],
        texture_path: impl Into<String>,
        layer: RenderLayer,
        z_index: i32,
    ) -> Self {
        Self {
            position: Vec2::new(position[0], position[1]),
            scale: Vec2::new(scale[0], scale[1]),
            texture_path: texture_path.into(),
            layer,
            z_index,
            hidden: false,
        }
    }

    pub fn render_sort_key(&self) -> (i32, i32) {
        (self.layer.order(), self.z_index)
    }

    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
}
