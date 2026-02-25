use std::collections::{HashMap, VecDeque};

use crate::{
    achievements::{AchievementNotification, AchievementSnapshotItem},
    audio::AudioEngine,
    game_object::DialogueBoxObject,
};
use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, RichText, Sense, Stroke, Ui,
};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use winit::{event::WindowEvent, window::Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    None,
    StartGame,
    SkipWait,
    ExitApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Audio,
    Text,
    Interface,
    Notifications,
}

impl SettingsTab {
    const fn title(self) -> &'static str {
        match self {
            Self::Audio => "Аудио",
            Self::Text => "Текст",
            Self::Interface => "Интерфейс",
            Self::Notifications => "Уведомления",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiThemePreset {
    DeepSea,
    Forest,
    Ember,
}

impl UiThemePreset {
    const fn title(self) -> &'static str {
        match self {
            Self::DeepSea => "Морская",
            Self::Forest => "Лесная",
            Self::Ember => "Янтарная",
        }
    }
}

#[derive(Debug, Clone)]
struct UiSettings {
    master_volume: f32,
    typewriter_sound_enabled: bool,
    typewriter_sound_volume: f32,
    typewriter_enabled: bool,
    typing_chars_per_second: f32,
    show_typing_caret: bool,
    show_speaker_name: bool,
    allow_dialogue_click_skip: bool,
    dialogue_text_size: f32,
    speaker_text_size: f32,
    dialogue_box_opacity: f32,
    dialogue_box_height_ratio: f32,
    dialogue_corner_radius: u8,
    ui_scale: f32,
    compact_menu_buttons: bool,
    menu_title_size: f32,
    menu_button_text_size: f32,
    animation_speed: f32,
    theme_preset: UiThemePreset,
    popup_enabled: bool,
    popup_duration: f32,
    show_achievement_descriptions: bool,
    achievement_list_spacing: f32,
    high_contrast_locked_achievements: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            typewriter_sound_enabled: true,
            typewriter_sound_volume: 0.20,
            typewriter_enabled: true,
            typing_chars_per_second: 40.0,
            show_typing_caret: true,
            show_speaker_name: true,
            allow_dialogue_click_skip: true,
            dialogue_text_size: 27.0,
            speaker_text_size: 21.0,
            dialogue_box_opacity: 0.92,
            dialogue_box_height_ratio: 0.16,
            dialogue_corner_radius: 12,
            ui_scale: 1.0,
            compact_menu_buttons: false,
            menu_title_size: 38.0,
            menu_button_text_size: 26.0,
            animation_speed: 1.0,
            theme_preset: UiThemePreset::DeepSea,
            popup_enabled: true,
            popup_duration: 3.8,
            show_achievement_descriptions: true,
            achievement_list_spacing: 8.0,
            high_contrast_locked_achievements: false,
        }
    }
}

#[derive(Clone, Copy)]
struct UiThemePalette {
    menu_fill: Color32,
    menu_stroke: Color32,
    menu_title: Color32,
    settings_fill: Color32,
    settings_stroke: Color32,
    settings_title: Color32,
    dialogue_fill_rgb: [u8; 3],
    dialogue_stroke: Color32,
    dialogue_speaker: Color32,
    dialogue_text: Color32,
    skip_ready: Color32,
    skip_wait: Color32,
    popup_fill: Color32,
    popup_stroke: Color32,
    popup_title: Color32,
    popup_name: Color32,
    popup_body: Color32,
}

pub struct DialogueUi {
    egui_ctx: egui::Context,
    egui_state: EguiWinitState,
    egui_renderer: Renderer,
    dialogue_objects: Vec<DialogueBoxObject>,
    dialogue_lookup: HashMap<String, usize>,
    // Per-dialogue character progress used by the typewriter effect.
    typing_progress: HashMap<String, f32>,
    typewriter_sound_id: Option<String>,
    // True when at least one new character appeared in this frame.
    typewriter_sound_pending: bool,
    main_menu_enabled: bool,
    settings_open: bool,
    settings_tab: SettingsTab,
    achievements_open: bool,
    achievements_snapshot: Vec<AchievementSnapshotItem>,
    achievement_notifications: VecDeque<AchievementNotification>,
    active_achievement_popup: Option<ActiveAchievementPopup>,
    settings: UiSettings,
}

struct ActiveAchievementPopup {
    notification: AchievementNotification,
    remaining: f32,
}

impl DialogueUi {
    pub fn new(
        window: &Window,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let egui_ctx = egui::Context::default();
        let egui_state = EguiWinitState::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let egui_renderer = Renderer::new(device, surface_format, Default::default());

        Self {
            egui_ctx,
            egui_state,
            egui_renderer,
            dialogue_objects: Vec::new(),
            dialogue_lookup: HashMap::new(),
            typing_progress: HashMap::new(),
            typewriter_sound_id: None,
            typewriter_sound_pending: false,
            main_menu_enabled: true,
            settings_open: false,
            settings_tab: SettingsTab::Audio,
            achievements_open: false,
            achievements_snapshot: Vec::new(),
            achievement_notifications: VecDeque::new(),
            active_achievement_popup: None,
            settings: UiSettings::default(),
        }
    }

    pub fn set_typewriter_sound(&mut self, sound_id: impl Into<String>, volume: f32) -> &mut Self {
        self.typewriter_sound_id = Some(sound_id.into());
        self.settings.typewriter_sound_volume = volume.clamp(0.0, 1.0);
        self
    }

    #[allow(dead_code)]
    pub fn clear_typewriter_sound(&mut self) -> &mut Self {
        self.typewriter_sound_id = None;
        self
    }

    pub fn set_main_menu_enabled(&mut self, enabled: bool) -> &mut Self {
        self.main_menu_enabled = enabled;
        if !enabled {
            self.settings_open = false;
            self.achievements_open = false;
        }
        self
    }

    pub fn set_achievements_snapshot(
        &mut self,
        achievements: Vec<AchievementSnapshotItem>,
    ) -> &mut Self {
        self.achievements_snapshot = achievements;
        self
    }

    pub fn enqueue_achievement_notifications(
        &mut self,
        notifications: Vec<AchievementNotification>,
    ) -> &mut Self {
        self.achievement_notifications.extend(notifications);
        self
    }

    pub fn has_active_achievement_popup(&self) -> bool {
        if !self.settings.popup_enabled {
            return false;
        }

        self.active_achievement_popup.is_some() || !self.achievement_notifications.is_empty()
    }

    pub fn apply_dialogue_object(&mut self, dialogue: DialogueBoxObject) {
        let key = dialogue.scene_key();

        if let Some(index) = self.dialogue_lookup.get(&key).copied() {
            let mut reset_typing = true;
            if let Some(existing) = self.dialogue_objects.get_mut(index) {
                // Restart typing if text changed or the dialogue became visible again.
                reset_typing =
                    existing.text != dialogue.text || existing.hidden && !dialogue.hidden;
                *existing = dialogue;
            }
            if reset_typing {
                self.typing_progress.insert(key.clone(), 0.0);
            }
            self.rebuild_dialogue_lookup();
            return;
        }

        self.dialogue_objects.push(dialogue);
        self.typing_progress.insert(key, 0.0);
        self.rebuild_dialogue_lookup();
    }

    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.egui_state.on_window_event(window, event).repaint
    }

    pub fn render(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        dt: f32,
        audio: Option<&mut AudioEngine>,
    ) -> UiCommand {
        self.typewriter_sound_pending = false;

        let egui_ctx = self.egui_ctx.clone();
        egui_ctx.set_pixels_per_point(self.settings.ui_scale.clamp(0.75, 1.6));

        let raw_input = self.egui_state.take_egui_input(window);
        let mut ui_command = UiCommand::None;
        let full_output = egui_ctx.run(raw_input, |ctx| {
            if self.main_menu_enabled {
                ui_command = self.draw_main_menu(ctx);
            } else if self.draw_dialogue_boxes(ctx, dt) {
                ui_command = UiCommand::SkipWait;
            }

            self.draw_achievement_popup(ctx, dt);
        });

        // Play at most one tick sound per frame if typing advanced.
        if self.typewriter_sound_pending && self.settings.typewriter_sound_enabled {
            if let (Some(sound_id), Some(audio)) = (self.typewriter_sound_id.as_deref(), audio) {
                let volume = self.settings.master_volume * self.settings.typewriter_sound_volume;
                if volume > 0.0 {
                    if let Err(err) = audio.play(sound_id, volume) {
                        eprintln!("typewriter sound playback failed: {err}");
                    }
                }
            }
        }

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        let pixels_per_point = egui_winit::pixels_per_point(&egui_ctx, window);
        let paint_jobs = egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        let size = window.inner_size();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.width.max(1), size.height.max(1)],
            pixels_per_point,
        };

        for (id, image_delta) in full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(device, queue, id, &image_delta);
        }

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut command_buffers = self.egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dialogue_ui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut render_pass = render_pass.forget_lifetime();
            self.egui_renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        command_buffers.push(encoder.finish());
        queue.submit(command_buffers);

        for id in full_output.textures_delta.free {
            self.egui_renderer.free_texture(&id);
        }

        ui_command
    }

    pub fn has_active_typewriter_animation(&self) -> bool {
        if self.main_menu_enabled || !self.settings.typewriter_enabled {
            return false;
        }

        // Used by the app loop to keep requesting redraw while text is still animating.
        self.dialogue_objects
            .iter()
            .filter(|dialogue| !dialogue.hidden)
            .any(|dialogue| {
                let key = dialogue.scene_key();
                let shown = self.typing_progress.get(&key).copied().unwrap_or(0.0);
                shown < dialogue.text.chars().count() as f32
            })
    }

    pub fn can_skip_wait(&self) -> bool {
        !self.has_active_typewriter_animation()
    }

    fn draw_dialogue_boxes(&mut self, ctx: &egui::Context, dt: f32) -> bool {
        let mut skip_requested = false;

        let visible_dialogues: Vec<_> = self
            .dialogue_objects
            .iter()
            .filter(|dialogue| !dialogue.hidden)
            .map(|dialogue| (dialogue.scene_key(), dialogue))
            .collect();

        if visible_dialogues.is_empty() {
            return false;
        }

        let palette = self.theme_palette();
        let viewport = ctx.viewport_rect();
        let max_width = (viewport.width() - 18.0).max(240.0);
        let box_width = (viewport.width() * 0.90).clamp(240.0, max_width);
        let box_height =
            (viewport.height() * self.settings.dialogue_box_height_ratio).clamp(104.0, 180.0);
        let x = viewport.left() + (viewport.width() - box_width) * 0.5;
        let mut y = viewport.bottom() - box_height - 14.0;

        let mut displayed_texts: Vec<String> = Vec::with_capacity(visible_dialogues.len());
        let mut all_dialogues_revealed = true;
        let anim_dt = dt.max(0.0) * self.settings.animation_speed.clamp(0.2, 2.0);

        for (key, dialogue) in &visible_dialogues {
            let total_chars = dialogue.text.chars().count();
            let shown_progress = self.typing_progress.entry(key.clone()).or_insert(0.0);
            let previous_chars = shown_progress.floor() as usize;

            if self.settings.typewriter_enabled {
                *shown_progress = (*shown_progress
                    + anim_dt * self.settings.typing_chars_per_second)
                    .min(total_chars as f32);
            } else {
                *shown_progress = total_chars as f32;
            }

            let shown_chars = shown_progress.floor() as usize;

            // If new characters were revealed this frame, schedule a typewriter tick.
            if shown_chars > previous_chars {
                self.typewriter_sound_pending = true;
            }

            // Render only the visible text prefix plus a caret while typing is active.
            let mut displayed_text: String = dialogue.text.chars().take(shown_chars).collect();
            if shown_chars < total_chars {
                if self.settings.show_typing_caret {
                    displayed_text.push('|');
                }
                all_dialogues_revealed = false;
            }

            displayed_texts.push(displayed_text);
        }

        let fill_alpha = (self.settings.dialogue_box_opacity.clamp(0.15, 1.0) * 255.0) as u8;

        for (index, (_key, dialogue)) in visible_dialogues.iter().enumerate() {
            let displayed_text = &displayed_texts[index];

            egui::Area::new(egui::Id::new(("dialogue_box", index)))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    ui.set_min_width(box_width);
                    ui.set_max_width(box_width);
                    ui.set_min_height(box_height);
                    ui.set_max_height(box_height);

                    let frame_response = Frame::new()
                        .inner_margin(Margin::symmetric(22, 14))
                        .fill(Color32::from_rgba_unmultiplied(
                            palette.dialogue_fill_rgb[0],
                            palette.dialogue_fill_rgb[1],
                            palette.dialogue_fill_rgb[2],
                            fill_alpha,
                        ))
                        .stroke(Stroke::new(2.0, palette.dialogue_stroke))
                        .corner_radius(CornerRadius::same(self.settings.dialogue_corner_radius))
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                if self.settings.show_speaker_name && !dialogue.speaker.is_empty() {
                                    ui.label(
                                        RichText::new(dialogue.speaker.as_str())
                                            .size(self.settings.speaker_text_size)
                                            .color(palette.dialogue_speaker),
                                    );
                                }
                                ui.label(
                                    RichText::new(displayed_text)
                                        .size(self.settings.dialogue_text_size)
                                        .color(palette.dialogue_text),
                                );
                                ui.separator();

                                let skip_enabled = all_dialogues_revealed
                                    && self.settings.allow_dialogue_click_skip;
                                let skip_color = if skip_enabled {
                                    palette.skip_ready
                                } else {
                                    palette.skip_wait
                                };
                                let skip_label = if all_dialogues_revealed {
                                    "Пропустить"
                                } else {
                                    "Печать..."
                                };
                                let skip_link = ui.add_enabled(
                                    skip_enabled,
                                    egui::Label::new(
                                        RichText::new(skip_label).size(18.0).color(skip_color),
                                    )
                                    .sense(Sense::click()),
                                );
                                if skip_link.clicked() {
                                    skip_requested = true;
                                }
                            });
                        });

                    let click_response = ui.interact(
                        frame_response.response.rect,
                        egui::Id::new(("dialogue_box_click", index)),
                        Sense::click(),
                    );
                    if all_dialogues_revealed
                        && self.settings.allow_dialogue_click_skip
                        && click_response.clicked()
                    {
                        skip_requested = true;
                    }
                });

            y -= box_height + 12.0;
        }

        skip_requested
    }

    fn draw_main_menu(&mut self, ctx: &egui::Context) -> UiCommand {
        let mut command = UiCommand::None;
        let palette = self.theme_palette();

        if !self.achievements_open && !self.settings_open {
            egui::Area::new(egui::Id::new("main_menu_root"))
                .order(egui::Order::Foreground)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    Frame::new()
                        .inner_margin(Margin::symmetric(26, 20))
                        .fill(palette.menu_fill)
                        .stroke(Stroke::new(2.0, palette.menu_stroke))
                        .corner_radius(CornerRadius::same(16))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Главное меню")
                                        .size(self.settings.menu_title_size)
                                        .color(palette.menu_title),
                                );
                                ui.add_space(12.0);

                                let button_size = if self.settings.compact_menu_buttons {
                                    egui::vec2(228.0, 40.0)
                                } else {
                                    egui::vec2(250.0, 48.0)
                                };

                                if ui
                                    .add_sized(
                                        button_size,
                                        egui::Button::new(
                                            RichText::new("Играть")
                                                .size(self.settings.menu_button_text_size),
                                        ),
                                    )
                                    .clicked()
                                {
                                    command = UiCommand::StartGame;
                                }

                                if ui
                                    .add_sized(
                                        button_size,
                                        egui::Button::new(
                                            RichText::new("Настройки")
                                                .size(self.settings.menu_button_text_size),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.settings_open = true;
                                    self.achievements_open = false;
                                }

                                if ui
                                    .add_sized(
                                        button_size,
                                        egui::Button::new(
                                            RichText::new("Достижения")
                                                .size(self.settings.menu_button_text_size),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.achievements_open = true;
                                    self.settings_open = false;
                                }

                                if ui
                                    .add_sized(
                                        button_size,
                                        egui::Button::new(
                                            RichText::new("Выход")
                                                .size(self.settings.menu_button_text_size),
                                        ),
                                    )
                                    .clicked()
                                {
                                    command = UiCommand::ExitApp;
                                }
                            });
                        });
                });
        }

        if self.settings_open {
            self.draw_settings_window(ctx, palette);
        }

        if self.achievements_open {
            self.draw_achievements_window(ctx);
        }

        command
    }

    fn draw_settings_window(&mut self, ctx: &egui::Context, palette: UiThemePalette) {
        let mut should_close = false;

        egui::Window::new("Настройки")
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_size([760.0, 560.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                Frame::new()
                    .inner_margin(Margin::symmetric(14, 12))
                    .fill(palette.settings_fill)
                    .stroke(Stroke::new(1.5, palette.settings_stroke))
                    .corner_radius(CornerRadius::same(14))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Гибкая настройка интерфейса")
                                .size(28.0)
                                .color(palette.settings_title),
                        );
                        ui.label(
                            RichText::new("Выбранные параметры применяются сразу.")
                                .size(16.0)
                                .color(Color32::from_rgb(176, 190, 201)),
                        );
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            self.draw_tab_button(ui, SettingsTab::Audio);
                            self.draw_tab_button(ui, SettingsTab::Text);
                            self.draw_tab_button(ui, SettingsTab::Interface);
                            self.draw_tab_button(ui, SettingsTab::Notifications);
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(8.0);

                        egui::ScrollArea::vertical().show(ui, |ui| match self.settings_tab {
                            SettingsTab::Audio => self.draw_audio_settings(ui),
                            SettingsTab::Text => self.draw_text_settings(ui),
                            SettingsTab::Interface => self.draw_interface_settings(ui),
                            SettingsTab::Notifications => self.draw_notification_settings(ui),
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new("Сбросить по умолчанию").size(18.0))
                                .clicked()
                            {
                                self.settings = UiSettings::default();
                            }

                            if ui
                                .button(RichText::new("Закрыть настройки").size(18.0))
                                .clicked()
                            {
                                should_close = true;
                            }
                        });
                    });
            });

        if should_close {
            self.settings_open = false;
        }
    }

    fn draw_tab_button(&mut self, ui: &mut Ui, tab: SettingsTab) {
        let is_active = self.settings_tab == tab;
        let bg = if is_active {
            Color32::from_rgb(52, 84, 112)
        } else {
            Color32::from_rgb(31, 39, 47)
        };

        Frame::new()
            .inner_margin(Margin::symmetric(8, 6))
            .fill(bg)
            .stroke(Stroke::new(1.0, Color32::from_rgb(94, 130, 160)))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                if ui
                    .selectable_label(is_active, RichText::new(tab.title()).size(18.0))
                    .clicked()
                {
                    self.settings_tab = tab;
                }
            });
    }

    fn draw_audio_settings(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Аудио").size(24.0));
        ui.add_space(6.0);

        ui.add(
            egui::Slider::new(&mut self.settings.master_volume, 0.0..=1.0).text("Общая громкость"),
        );
        ui.checkbox(
            &mut self.settings.typewriter_sound_enabled,
            "Включить звук печати",
        );
        ui.add_enabled(
            self.settings.typewriter_sound_enabled,
            egui::Slider::new(&mut self.settings.typewriter_sound_volume, 0.0..=1.0)
                .text("Громкость звука печати"),
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new("Подсказка: для тихого режима поставьте 0.0 в 'Общая громкость'.")
                .size(15.0)
                .color(Color32::from_rgb(155, 168, 181)),
        );
    }

    fn draw_text_settings(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Текст и диалоги").size(24.0));
        ui.add_space(6.0);

        ui.checkbox(&mut self.settings.typewriter_enabled, "Эффект печати");
        ui.add_enabled(
            self.settings.typewriter_enabled,
            egui::Slider::new(&mut self.settings.typing_chars_per_second, 8.0..=120.0)
                .text("Скорость печати (симв/с)"),
        );
        ui.checkbox(
            &mut self.settings.show_typing_caret,
            "Показывать курсор печати",
        );
        ui.checkbox(
            &mut self.settings.allow_dialogue_click_skip,
            "Разрешить пропуск кликом",
        );
        ui.checkbox(
            &mut self.settings.show_speaker_name,
            "Показывать имя говорящего",
        );
        ui.add(
            egui::Slider::new(&mut self.settings.speaker_text_size, 14.0..=32.0)
                .text("Размер имени"),
        );
        ui.add(
            egui::Slider::new(&mut self.settings.dialogue_text_size, 18.0..=42.0)
                .text("Размер текста"),
        );
    }

    fn draw_interface_settings(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Интерфейс").size(24.0));
        ui.add_space(6.0);

        ui.add(egui::Slider::new(&mut self.settings.ui_scale, 0.75..=1.60).text("Масштаб UI"));
        ui.checkbox(
            &mut self.settings.compact_menu_buttons,
            "Компактные кнопки меню",
        );
        ui.add(
            egui::Slider::new(&mut self.settings.menu_title_size, 28.0..=56.0)
                .text("Размер заголовка меню"),
        );
        ui.add(
            egui::Slider::new(&mut self.settings.menu_button_text_size, 18.0..=34.0)
                .text("Размер текста кнопок"),
        );
        ui.add(
            egui::Slider::new(&mut self.settings.dialogue_box_opacity, 0.2..=1.0)
                .text("Прозрачность диалогового окна"),
        );
        ui.add(
            egui::Slider::new(&mut self.settings.dialogue_box_height_ratio, 0.12..=0.26)
                .text("Высота диалогового окна"),
        );
        ui.add(
            egui::Slider::new(&mut self.settings.dialogue_corner_radius, 4..=24)
                .text("Скругление диалогового окна"),
        );
        ui.add(
            egui::Slider::new(&mut self.settings.animation_speed, 0.2..=2.0)
                .text("Скорость анимаций"),
        );

        ui.add_space(6.0);
        ui.label(RichText::new("Цветовая тема").size(20.0));
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.settings.theme_preset,
                UiThemePreset::DeepSea,
                UiThemePreset::DeepSea.title(),
            );
            ui.selectable_value(
                &mut self.settings.theme_preset,
                UiThemePreset::Forest,
                UiThemePreset::Forest.title(),
            );
            ui.selectable_value(
                &mut self.settings.theme_preset,
                UiThemePreset::Ember,
                UiThemePreset::Ember.title(),
            );
        });
    }

    fn draw_notification_settings(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Уведомления и достижения").size(24.0));
        ui.add_space(6.0);

        ui.checkbox(
            &mut self.settings.popup_enabled,
            "Показывать всплывающее окно достижения",
        );
        ui.add_enabled(
            self.settings.popup_enabled,
            egui::Slider::new(&mut self.settings.popup_duration, 1.0..=8.0)
                .text("Длительность попапа (сек.)"),
        );
        ui.checkbox(
            &mut self.settings.show_achievement_descriptions,
            "Показывать описание в списке достижений",
        );
        ui.checkbox(
            &mut self.settings.high_contrast_locked_achievements,
            "Контрастные заблокированные карточки",
        );
        ui.add(
            egui::Slider::new(&mut self.settings.achievement_list_spacing, 2.0..=18.0)
                .text("Отступ между карточками достижений"),
        );
    }

    fn draw_achievements_window(&mut self, ctx: &egui::Context) {
        let mut should_close = false;
        let unlocked_count = self
            .achievements_snapshot
            .iter()
            .filter(|achievement| achievement.unlocked)
            .count();
        let total_count = self.achievements_snapshot.len();

        egui::Window::new("Достижения")
            .anchor(Align2::CENTER_BOTTOM, [0.0, -48.0])
            .default_size([540.0, 440.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("Открыто: {unlocked_count}/{total_count}")).size(22.0),
                );
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for achievement in &self.achievements_snapshot {
                            let (status, border, title_color, body_color, fill) =
                                if achievement.unlocked {
                                    (
                                        "Открыто",
                                        Color32::from_rgb(114, 185, 113),
                                        Color32::from_rgb(222, 250, 201),
                                        Color32::from_rgb(214, 238, 207),
                                        Color32::from_rgba_unmultiplied(24, 52, 24, 214),
                                    )
                                } else if self.settings.high_contrast_locked_achievements {
                                    (
                                        "Заблокировано",
                                        Color32::from_rgb(154, 93, 93),
                                        Color32::from_rgb(231, 191, 191),
                                        Color32::from_rgb(223, 175, 175),
                                        Color32::from_rgba_unmultiplied(48, 22, 22, 220),
                                    )
                                } else {
                                    (
                                        "Заблокировано",
                                        Color32::from_rgb(94, 109, 122),
                                        Color32::from_rgb(148, 165, 176),
                                        Color32::from_rgb(128, 140, 149),
                                        Color32::from_rgba_unmultiplied(19, 24, 30, 214),
                                    )
                                };

                            Frame::new()
                                .inner_margin(Margin::symmetric(14, 10))
                                .fill(fill)
                                .stroke(Stroke::new(1.0, border))
                                .corner_radius(CornerRadius::same(10))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("{} [{}]", achievement.name, status))
                                            .size(20.0)
                                            .color(title_color),
                                    );

                                    if self.settings.show_achievement_descriptions {
                                        ui.label(
                                            RichText::new(achievement.description.as_str())
                                                .size(17.0)
                                                .color(body_color),
                                        );
                                    }
                                });

                            ui.add_space(self.settings.achievement_list_spacing);
                        }
                    });

                ui.add_space(4.0);
                if ui
                    .button(RichText::new("Закрыть список достижений").size(19.0))
                    .clicked()
                {
                    should_close = true;
                }
            });

        if should_close {
            self.achievements_open = false;
        }
    }

    fn draw_achievement_popup(&mut self, ctx: &egui::Context, dt: f32) {
        if !self.settings.popup_enabled {
            self.active_achievement_popup = None;
            self.achievement_notifications.clear();
            return;
        }

        if self.active_achievement_popup.is_none() {
            if let Some(next) = self.achievement_notifications.pop_front() {
                self.active_achievement_popup = Some(ActiveAchievementPopup {
                    notification: next,
                    remaining: self.settings.popup_duration.clamp(1.0, 8.0),
                });
            }
        }

        let Some(active) = self.active_achievement_popup.as_ref() else {
            return;
        };

        let palette = self.theme_palette();
        egui::Area::new(egui::Id::new("achievement_popup"))
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_TOP, [-18.0, 18.0])
            .show(ctx, |ui| {
                ui.set_max_width(420.0);
                Frame::new()
                    .inner_margin(Margin::symmetric(16, 12))
                    .fill(palette.popup_fill)
                    .stroke(Stroke::new(2.0, palette.popup_stroke))
                    .corner_radius(CornerRadius::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Достижение получено!")
                                .size(20.0)
                                .color(palette.popup_title),
                        );
                        ui.label(
                            RichText::new(active.notification.name.as_str())
                                .size(24.0)
                                .color(palette.popup_name),
                        );
                        ui.label(
                            RichText::new(active.notification.description.as_str())
                                .size(18.0)
                                .color(palette.popup_body),
                        );
                    });
            });

        let time_step = if dt > 0.0 {
            dt * self.settings.animation_speed.clamp(0.2, 2.0)
        } else {
            1.0 / 60.0
        };

        if let Some(active) = self.active_achievement_popup.as_mut() {
            active.remaining -= time_step;
            if active.remaining <= 0.0 {
                self.active_achievement_popup = None;
            }
        }
    }

    fn theme_palette(&self) -> UiThemePalette {
        match self.settings.theme_preset {
            UiThemePreset::DeepSea => UiThemePalette {
                menu_fill: Color32::from_rgba_unmultiplied(8, 18, 30, 238),
                menu_stroke: Color32::from_rgb(120, 140, 90),
                menu_title: Color32::from_rgb(244, 228, 157),
                settings_fill: Color32::from_rgba_unmultiplied(10, 19, 30, 238),
                settings_stroke: Color32::from_rgb(91, 132, 164),
                settings_title: Color32::from_rgb(220, 234, 248),
                dialogue_fill_rgb: [10, 20, 28],
                dialogue_stroke: Color32::from_rgb(118, 160, 186),
                dialogue_speaker: Color32::from_rgb(166, 124, 208),
                dialogue_text: Color32::from_rgb(244, 229, 178),
                skip_ready: Color32::from_rgb(132, 194, 210),
                skip_wait: Color32::from_rgb(96, 106, 112),
                popup_fill: Color32::from_rgba_unmultiplied(22, 50, 19, 235),
                popup_stroke: Color32::from_rgb(132, 219, 104),
                popup_title: Color32::from_rgb(236, 255, 210),
                popup_name: Color32::from_rgb(212, 255, 173),
                popup_body: Color32::from_rgb(198, 232, 178),
            },
            UiThemePreset::Forest => UiThemePalette {
                menu_fill: Color32::from_rgba_unmultiplied(13, 26, 17, 238),
                menu_stroke: Color32::from_rgb(113, 162, 96),
                menu_title: Color32::from_rgb(226, 246, 175),
                settings_fill: Color32::from_rgba_unmultiplied(14, 27, 18, 238),
                settings_stroke: Color32::from_rgb(104, 164, 109),
                settings_title: Color32::from_rgb(216, 242, 208),
                dialogue_fill_rgb: [17, 28, 18],
                dialogue_stroke: Color32::from_rgb(111, 168, 116),
                dialogue_speaker: Color32::from_rgb(170, 216, 129),
                dialogue_text: Color32::from_rgb(238, 247, 206),
                skip_ready: Color32::from_rgb(154, 222, 141),
                skip_wait: Color32::from_rgb(97, 120, 97),
                popup_fill: Color32::from_rgba_unmultiplied(17, 47, 22, 235),
                popup_stroke: Color32::from_rgb(111, 219, 120),
                popup_title: Color32::from_rgb(220, 255, 219),
                popup_name: Color32::from_rgb(190, 255, 183),
                popup_body: Color32::from_rgb(181, 230, 175),
            },
            UiThemePreset::Ember => UiThemePalette {
                menu_fill: Color32::from_rgba_unmultiplied(33, 20, 14, 238),
                menu_stroke: Color32::from_rgb(196, 134, 84),
                menu_title: Color32::from_rgb(255, 224, 175),
                settings_fill: Color32::from_rgba_unmultiplied(31, 20, 14, 238),
                settings_stroke: Color32::from_rgb(189, 125, 73),
                settings_title: Color32::from_rgb(255, 219, 189),
                dialogue_fill_rgb: [30, 19, 12],
                dialogue_stroke: Color32::from_rgb(204, 140, 91),
                dialogue_speaker: Color32::from_rgb(255, 177, 120),
                dialogue_text: Color32::from_rgb(255, 227, 194),
                skip_ready: Color32::from_rgb(240, 182, 126),
                skip_wait: Color32::from_rgb(137, 105, 84),
                popup_fill: Color32::from_rgba_unmultiplied(52, 31, 14, 235),
                popup_stroke: Color32::from_rgb(241, 162, 86),
                popup_title: Color32::from_rgb(255, 231, 205),
                popup_name: Color32::from_rgb(255, 208, 156),
                popup_body: Color32::from_rgb(239, 200, 163),
            },
        }
    }

    fn rebuild_dialogue_lookup(&mut self) {
        self.dialogue_lookup.clear();
        for (index, dialogue) in self.dialogue_objects.iter().enumerate() {
            self.dialogue_lookup.insert(dialogue.scene_key(), index);
        }
        self.typing_progress
            .retain(|key, _| self.dialogue_lookup.contains_key(key));
    }
}
