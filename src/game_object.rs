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
    pub id: Option<String>,
    pub position: Vec2,
    pub scale: Vec2,
    pub texture_path: String,
    pub layer: RenderLayer,
    pub z_index: i32,
    pub hidden: bool,
}

#[derive(Clone, Debug)]
pub struct DialogueBoxObject {
    pub id: Option<String>,
    pub speaker: String,
    pub text: String,
    pub hidden: bool,
}

impl DialogueBoxObject {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: None,
            speaker: "Lena".to_string(),
            text: text.into(),
            hidden: false,
        }
    }

    #[allow(dead_code)]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = speaker.into();
        self
    }

    #[allow(dead_code)]
    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn scene_key(&self) -> String {
        if let Some(id) = &self.id {
            return format!("id:{id}");
        }

        format!("auto:{}:{}", self.speaker, self.text)
    }
}

#[derive(Clone, Debug)]
pub enum SceneObject {
    Sprite(GameObject2D),
    Dialogue(DialogueBoxObject),
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
            id: None,
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

    #[allow(dead_code)]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn scene_key(&self) -> String {
        if let Some(id) = &self.id {
            return format!("id:{id}");
        }

        format!(
            "auto:{}:{}:{}:{}:{}:{}:{}",
            self.texture_path,
            self.position.x.to_bits(),
            self.position.y.to_bits(),
            self.scale.x.to_bits(),
            self.scale.y.to_bits(),
            self.layer.order(),
            self.z_index,
        )
    }
}

impl From<GameObject2D> for SceneObject {
    fn from(value: GameObject2D) -> Self {
        Self::Sprite(value)
    }
}

impl From<DialogueBoxObject> for SceneObject {
    fn from(value: DialogueBoxObject) -> Self {
        Self::Dialogue(value)
    }
}
