use std::collections::{HashSet, VecDeque};

use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    SkipWait,
    Exit,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum InputEvent {
    KeyPressed(KeyCode),
    KeyReleased(KeyCode),
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
    CursorMoved { x: f32, y: f32 },
    MouseWheel { delta_y: f32 },
}

pub struct ActionMap {
    skip_wait_keys: Vec<KeyCode>,
    exit_keys: Vec<KeyCode>,
}

impl Default for ActionMap {
    fn default() -> Self {
        Self {
            skip_wait_keys: vec![KeyCode::Space, KeyCode::Enter],
            exit_keys: vec![KeyCode::Escape],
        }
    }
}

impl ActionMap {
    pub fn just_pressed(&self, action: Action, input: &InputState) -> bool {
        let keys = match action {
            Action::SkipWait => &self.skip_wait_keys,
            Action::Exit => &self.exit_keys,
        };

        keys.iter().any(|key| input.was_key_just_pressed(*key))
    }
}

#[derive(Default)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    just_pressed_keys: HashSet<KeyCode>,
    just_released_keys: HashSet<KeyCode>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    just_pressed_mouse_buttons: HashSet<MouseButton>,
    just_released_mouse_buttons: HashSet<MouseButton>,
    events: VecDeque<InputEvent>,
    cursor_position: Option<(f32, f32)>,
}

impl InputState {
    pub fn on_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return false;
                };

                match event.state {
                    ElementState::Pressed => {
                        // "just pressed" only on first press (ignore key repeat).
                        if self.pressed_keys.insert(code) {
                            self.just_pressed_keys.insert(code);
                            self.events.push_back(InputEvent::KeyPressed(code));
                            return true;
                        }
                    }
                    ElementState::Released => {
                        self.pressed_keys.remove(&code);
                        self.just_released_keys.insert(code);
                        self.events.push_back(InputEvent::KeyReleased(code));
                        return true;
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    if self.pressed_mouse_buttons.insert(*button) {
                        self.just_pressed_mouse_buttons.insert(*button);
                        self.events.push_back(InputEvent::MousePressed(*button));
                        return true;
                    }
                }
                ElementState::Released => {
                    self.pressed_mouse_buttons.remove(button);
                    self.just_released_mouse_buttons.insert(*button);
                    self.events.push_back(InputEvent::MouseReleased(*button));
                    return true;
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x as f32, position.y as f32);
                self.cursor_position = Some(pos);
                self.events
                    .push_back(InputEvent::CursorMoved { x: pos.0, y: pos.1 });
                return true;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                self.events.push_back(InputEvent::MouseWheel { delta_y });
                return true;
            }
            _ => {}
        }

        false
    }

    #[allow(dead_code)]
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn was_key_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key)
    }

    #[allow(dead_code)]
    pub fn was_key_just_released(&self, key: KeyCode) -> bool {
        self.just_released_keys.contains(&key)
    }

    #[allow(dead_code)]
    pub fn cursor_position(&self) -> Option<(f32, f32)> {
        self.cursor_position
    }

    #[allow(dead_code)]
    pub fn events(&self) -> &VecDeque<InputEvent> {
        &self.events
    }

    pub fn end_frame(&mut self) {
        self.just_pressed_keys.clear();
        self.just_released_keys.clear();
        self.just_pressed_mouse_buttons.clear();
        self.just_released_mouse_buttons.clear();
        self.events.clear();
    }
}
