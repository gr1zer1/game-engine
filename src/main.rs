use std::{sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

mod state;
use state::State;
mod dialogue_ui;
mod game_object;
mod input;
mod scene_objects;
mod scene_script;
mod tex;
use dialogue_ui::DialogueUi;
use input::{Action, ActionMap, InputState};
use scene_script::SceneTimeline;
use tex::Tex;

struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    tex: Option<Tex>,
    dialogue_ui: Option<DialogueUi>,
    scene_timeline: Option<SceneTimeline>,
    input: InputState,
    action_map: ActionMap,
    last_frame_time: Option<Instant>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            state: None,
            tex: None,
            dialogue_ui: None,
            scene_timeline: None,
            input: InputState::default(),
            action_map: ActionMap::default(),
            last_frame_time: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("ok");

        let window = Some(Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        ));

        self.window = window.clone();

        let state_ = pollster::block_on(State::new(window.unwrap()));

        self.state = Some(state_.unwrap());

        State::resumed(&mut self.state.as_mut().unwrap());

        if let Some(state) = &self.state {
            let mut tex = Tex::init(
                &state.config.as_ref().unwrap(),
                &state.adapter,
                &state.device,
                &state.queue,
            );
            let mut dialogue_ui = DialogueUi::new(
                self.window.as_ref().unwrap().as_ref(),
                &state.device,
                state.config.as_ref().unwrap().format,
            );

            let mut scene_timeline = SceneTimeline::new(scene_objects::read_initial_scene_script());
            scene_timeline
                .update(0.0, &state.device, &state.queue, &mut tex, &mut dialogue_ui)
                .expect("failed to initialize scene script");

            self.tex = Some(tex);
            self.dialogue_ui = Some(dialogue_ui);
            self.scene_timeline = Some(scene_timeline);
            self.last_frame_time = Some(Instant::now());
        }

        // Request initial redraw
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.input.on_window_event(&event) {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        if let (Some(dialogue_ui), Some(window)) = (self.dialogue_ui.as_mut(), self.window.as_ref())
        {
            if dialogue_ui.on_window_event(window.as_ref(), &event) {
                window.request_redraw();
            }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(tex), Some(dialogue_ui), Some(window)) = (
                    self.state.as_ref(),
                    self.tex.as_mut(),
                    self.dialogue_ui.as_mut(),
                    self.window.as_ref(),
                ) {
                    if self.action_map.just_pressed(Action::Exit, &self.input) {
                        event_loop.exit();
                        return;
                    }

                    if self.action_map.just_pressed(Action::SkipWait, &self.input) {
                        if let Some(scene_timeline) = self.scene_timeline.as_mut() {
                            scene_timeline.skip_wait();
                        }
                    }

                    let now = Instant::now();
                    let dt = self
                        .last_frame_time
                        .map(|last| (now - last).as_secs_f32())
                        .unwrap_or(0.0);
                    self.last_frame_time = Some(now);

                    if let Some(scene_timeline) = self.scene_timeline.as_mut() {
                        scene_timeline
                            .update(dt, &state.device, &state.queue, tex, dialogue_ui)
                            .expect("failed to update scene script");
                    }

                    // Получаем текущий кадр из поверхности окна
                    let frame = state
                        .surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");

                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    // Рендерим сцену и диалоговый UI в этот кадр
                    tex.render(&view, &state.device, &state.queue);
                    dialogue_ui.render(window.as_ref(), &state.device, &state.queue, &view);

                    // Показываем кадр на экране
                    frame.present();

                    if self
                        .scene_timeline
                        .as_ref()
                        .is_some_and(|timeline| !timeline.is_finished())
                    {
                        window.request_redraw();
                    }

                    self.input.end_frame();
                }
            }

            WindowEvent::Resized(new_size) => {
                if let Some(state) = &mut self.state {
                    if let Some(config) = &mut state.config {
                        config.width = new_size.width.max(1);
                        config.height = new_size.height.max(1);
                        state.surface.configure(&state.device, config);

                        if let Some(tex) = self.tex.as_mut() {
                            tex.resize(config, &state.device, &state.queue);
                        }
                    }
                    state.redraw();
                }
            }

            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
