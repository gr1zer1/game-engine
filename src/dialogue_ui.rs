use std::collections::HashMap;

use crate::{audio::AudioEngine, game_object::DialogueBoxObject};
use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use winit::{event::WindowEvent, window::Window};

pub struct DialogueUi {
    egui_ctx: egui::Context,
    egui_state: EguiWinitState,
    egui_renderer: Renderer,
    dialogue_objects: Vec<DialogueBoxObject>,
    dialogue_lookup: HashMap<String, usize>,
    // Per-dialogue character progress used by the typewriter effect.
    typing_progress: HashMap<String, f32>,
    typing_chars_per_second: f32,
    typewriter_sound_id: Option<String>,
    typewriter_sound_volume: f32,
    // True when at least one new character appeared in this frame.
    typewriter_sound_pending: bool,
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
            typing_chars_per_second: 40.0,
            typewriter_sound_id: None,
            typewriter_sound_volume: 0.20,
            typewriter_sound_pending: false,
        }
    }

    pub fn set_typewriter_sound(&mut self, sound_id: impl Into<String>, volume: f32) -> &mut Self {
        self.typewriter_sound_id = Some(sound_id.into());
        self.typewriter_sound_volume = volume.max(0.0);
        self
    }

    #[allow(dead_code)]
    pub fn clear_typewriter_sound(&mut self) -> &mut Self {
        self.typewriter_sound_id = None;
        self
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
    ) {
        self.typewriter_sound_pending = false;

        let egui_ctx = self.egui_ctx.clone();
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = egui_ctx.run(raw_input, |ctx| self.draw_dialogue_boxes(ctx, dt));

        // Play at most one tick sound per frame if typing advanced.
        if self.typewriter_sound_pending {
            if let (Some(sound_id), Some(audio)) = (self.typewriter_sound_id.as_deref(), audio) {
                if let Err(err) = audio.play(sound_id, self.typewriter_sound_volume) {
                    eprintln!("typewriter sound playback failed: {err}");
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
    }

    pub fn has_active_typewriter_animation(&self) -> bool {
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

    fn draw_dialogue_boxes(&mut self, ctx: &egui::Context, dt: f32) {
        let visible_dialogues: Vec<_> = self
            .dialogue_objects
            .iter()
            .filter(|dialogue| !dialogue.hidden)
            .map(|dialogue| (dialogue.scene_key(), dialogue))
            .collect();

        if visible_dialogues.is_empty() {
            return;
        }

        let viewport = ctx.viewport_rect();
        let box_width = viewport.width() * 0.86;
        let box_height = (viewport.height() * 0.22).max(120.0);
        let x = viewport.left() + (viewport.width() - box_width) * 0.5;
        let mut y = viewport.bottom() - box_height - 20.0;

        for (index, (key, dialogue)) in visible_dialogues.iter().enumerate() {
            let total_chars = dialogue.text.chars().count();
            let shown_progress = self.typing_progress.entry(key.clone()).or_insert(0.0);
            let previous_chars = shown_progress.floor() as usize;
            *shown_progress = (*shown_progress + dt.max(0.0) * self.typing_chars_per_second)
                .min(total_chars as f32);
            let shown_chars = shown_progress.floor() as usize;

            // If new characters were revealed this frame, schedule a typewriter tick.
            if shown_chars > previous_chars {
                self.typewriter_sound_pending = true;
            }

            // Render only the visible text prefix plus a caret while typing is active.
            let mut displayed_text: String = dialogue.text.chars().take(shown_chars).collect();
            if shown_chars < total_chars {
                displayed_text.push('|');
            }

            egui::Area::new(egui::Id::new(("dialogue_box", index)))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    ui.set_min_width(box_width);
                    ui.set_max_width(box_width);
                    ui.set_min_height(box_height);

                    Frame::new()
                        .inner_margin(Margin::symmetric(24, 14))
                        .fill(Color32::from_rgba_unmultiplied(8, 18, 30, 228))
                        .stroke(Stroke::new(1.5, Color32::from_rgb(120, 140, 90)))
                        .corner_radius(CornerRadius::same(14))
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                ui.spacing_mut().item_spacing.y = 6.0;
                                if !dialogue.speaker.is_empty() {
                                    ui.label(
                                        RichText::new(dialogue.speaker.as_str())
                                            .size(24.0)
                                            .color(Color32::from_rgb(180, 115, 255)),
                                    );
                                }
                                ui.label(
                                    RichText::new(displayed_text)
                                        .size(31.0)
                                        .color(Color32::from_rgb(244, 228, 157)),
                                );
                            });
                        });
                });

            y -= box_height + 12.0;
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
