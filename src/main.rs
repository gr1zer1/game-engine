use std::{sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

mod state;
use state::State;
mod achievements;
mod audio;
mod dialogue_ui;
mod game_object;
mod input;
mod scene_objects;
mod scene_script;
mod scripts;
mod tex;
use achievements::AchievementManager;
use audio::AudioEngine;
use dialogue_ui::{DialogueUi, UiCommand};
use input::{Action, ActionMap, InputState};
use scene_script::{SceneRunner, ScriptContext, ScriptSignal};
use tex::Tex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    MainMenu,
    InGame,
}

struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    tex: Option<Tex>,
    dialogue_ui: Option<DialogueUi>,
    audio: Option<AudioEngine>,
    achievements: Option<AchievementManager>,
    scene_runner: Option<SceneRunner>,
    input: InputState,
    action_map: ActionMap,
    last_frame_time: Option<Instant>,
    mode: AppMode,
    scene_bootstrapped: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            state: None,
            tex: None,
            dialogue_ui: None,
            audio: None,
            achievements: None,
            scene_runner: None,
            input: InputState::default(),
            action_map: ActionMap::default(),
            last_frame_time: None,
            mode: AppMode::MainMenu,
            scene_bootstrapped: false,
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
            let tex = Tex::init(
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
            let mut audio = match AudioEngine::new() {
                Ok(audio) => Some(audio),
                Err(err) => {
                    eprintln!("audio disabled: {err}");
                    None
                }
            };
            if let Some(audio_engine) = audio.as_mut() {
                // Built-in short blip used by dialogue typewriter.
                audio_engine.register_tone("dialogue_typewriter", 1240, 18);
                dialogue_ui.set_typewriter_sound("dialogue_typewriter", 0.16);

                // Optional external override: place your own clip at assets/sfx/type_tick.wav.
                if audio_engine
                    .register_sound_file("dialogue_typewriter", "assets/sfx/type_tick.wav")
                    .is_ok()
                {
                    dialogue_ui.set_typewriter_sound("dialogue_typewriter", 0.20);
                }
            }

            let scene_runner =
                SceneRunner::with_scripts(scene_objects::create_initial_scene_scripts());
            let achievements_path = scripts::achievements_catalog::DEFAULT_ACHIEVEMENTS_PATH;
            if let Err(err) =
                scripts::achievements_catalog::ensure_achievements_json_exists(achievements_path)
            {
                eprintln!("failed to prepare achievements catalog: {err}");
            }
            let achievements = AchievementManager::load_from_json_file(achievements_path)
                .or_else(|err| {
                    eprintln!("failed to load achievements json: {err}");
                    AchievementManager::from_definitions(
                        scripts::achievements_catalog::create_all_achievements(),
                    )
                })
                .unwrap_or_else(|err| {
                    eprintln!("failed to create fallback achievements catalog: {err}");
                    AchievementManager::from_definitions(Vec::new())
                        .expect("empty achievements catalog should be valid")
                });

            dialogue_ui.set_achievements_snapshot(achievements.snapshot());
            dialogue_ui.set_main_menu_enabled(true);

            self.tex = Some(tex);
            self.dialogue_ui = Some(dialogue_ui);
            self.audio = audio;
            self.achievements = Some(achievements);
            self.scene_runner = Some(scene_runner);
            self.last_frame_time = Some(Instant::now());
            self.mode = AppMode::MainMenu;
            self.scene_bootstrapped = false;
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
                if let (
                    Some(state),
                    Some(tex),
                    Some(dialogue_ui),
                    Some(window),
                    Some(achievements),
                ) = (
                    self.state.as_ref(),
                    self.tex.as_mut(),
                    self.dialogue_ui.as_mut(),
                    self.window.as_ref(),
                    self.achievements.as_mut(),
                ) {
                    if self.action_map.just_pressed(Action::Exit, &self.input) {
                        event_loop.exit();
                        return;
                    }

                    if matches!(self.mode, AppMode::InGame)
                        && self.action_map.just_pressed(Action::SkipWait, &self.input)
                        && dialogue_ui.can_skip_wait()
                    {
                        if let Some(scene_runner) = self.scene_runner.as_mut() {
                            // Broadcast to all scripts (used for dialogue skip/close behavior).
                            scene_runner.send_signal(ScriptSignal::SkipWait);
                        }
                    }

                    let dt = if matches!(self.mode, AppMode::InGame) {
                        let now = Instant::now();
                        let dt = self
                            .last_frame_time
                            .map(|last| (now - last).as_secs_f32())
                            .unwrap_or(0.0);
                        self.last_frame_time = Some(now);
                        dt
                    } else {
                        0.0
                    };

                    if matches!(self.mode, AppMode::InGame) {
                        if let Some(scene_runner) = self.scene_runner.as_mut() {
                            let mut script_context = ScriptContext {
                                device: &state.device,
                                queue: &state.queue,
                                tex,
                                dialogue_ui,
                                achievements,
                                audio: self.audio.as_mut(),
                            };
                            // Per-frame lifecycle update for all active scripts.
                            scene_runner
                                .update(dt, &mut script_context)
                                .expect("failed to update scene script");
                        }
                    }

                    dialogue_ui.set_achievements_snapshot(achievements.snapshot());
                    dialogue_ui
                        .enqueue_achievement_notifications(achievements.take_notifications());

                    // Acquire the current frame from the window surface.
                    let frame = state
                        .surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");

                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    // Render the scene and dialogue UI into this frame.
                    tex.render(&view, &state.device, &state.queue);
                    let audio = self.audio.as_mut();
                    let ui_command = dialogue_ui.render(
                        window.as_ref(),
                        &state.device,
                        &state.queue,
                        &view,
                        dt,
                        audio,
                    );

                    // Present the frame on screen.
                    frame.present();

                    match ui_command {
                        UiCommand::None => {}
                        UiCommand::StartGame => {
                            if !self.scene_bootstrapped {
                                if let Some(scene_runner) = self.scene_runner.as_mut() {
                                    let mut script_context = ScriptContext {
                                        device: &state.device,
                                        queue: &state.queue,
                                        tex,
                                        dialogue_ui,
                                        achievements,
                                        audio: self.audio.as_mut(),
                                    };
                                    scene_runner
                                        .update(0.0, &mut script_context)
                                        .expect("failed to initialize scene script");
                                }
                                self.scene_bootstrapped = true;
                            }
                            self.mode = AppMode::InGame;
                            dialogue_ui.set_main_menu_enabled(false);
                            self.last_frame_time = Some(Instant::now());
                            window.request_redraw();
                        }
                        UiCommand::SkipWait => {
                            if matches!(self.mode, AppMode::InGame) && dialogue_ui.can_skip_wait() {
                                if let Some(scene_runner) = self.scene_runner.as_mut() {
                                    scene_runner.send_signal(ScriptSignal::SkipWait);
                                }
                                window.request_redraw();
                            }
                        }
                        UiCommand::ExitApp => {
                            event_loop.exit();
                            return;
                        }
                    }

                    let achievements_path =
                        scripts::achievements_catalog::DEFAULT_ACHIEVEMENTS_PATH;
                    if let Err(err) = achievements.save_to_json_file(achievements_path) {
                        eprintln!("failed to save achievements progress: {err}");
                    }

                    let has_achievement_popup = dialogue_ui.has_active_achievement_popup();
                    if matches!(self.mode, AppMode::InGame) {
                        let scripts_are_running = self
                            .scene_runner
                            .as_ref()
                            .is_some_and(|runner| !runner.is_finished());
                        let dialogue_is_animating = dialogue_ui.has_active_typewriter_animation();

                        if scripts_are_running || dialogue_is_animating || has_achievement_popup {
                            window.request_redraw();
                        }
                    } else if has_achievement_popup {
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
