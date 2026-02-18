use std::collections::HashMap;

use crate::game_object::DialogueBoxObject;
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
        }
    }

    pub fn apply_dialogue_object(&mut self, dialogue: DialogueBoxObject) {
        let key = dialogue.scene_key();

        if let Some(index) = self.dialogue_lookup.get(&key).copied() {
            if let Some(existing) = self.dialogue_objects.get_mut(index) {
                *existing = dialogue;
            }
            self.rebuild_dialogue_lookup();
            return;
        }

        self.dialogue_objects.push(dialogue);
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
    ) {
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self
            .egui_ctx
            .run(raw_input, |ctx| self.draw_dialogue_boxes(ctx));

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        let pixels_per_point = egui_winit::pixels_per_point(&self.egui_ctx, window);
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, pixels_per_point);
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

    fn draw_dialogue_boxes(&self, ctx: &egui::Context) {
        let visible_dialogues: Vec<_> = self
            .dialogue_objects
            .iter()
            .filter(|dialogue| !dialogue.hidden)
            .collect();

        if visible_dialogues.is_empty() {
            return;
        }

        let viewport = ctx.viewport_rect();
        let box_width = viewport.width() * 0.86;
        let box_height = (viewport.height() * 0.22).max(120.0);
        let x = viewport.left() + (viewport.width() - box_width) * 0.5;
        let mut y = viewport.bottom() - box_height - 20.0;

        for (index, dialogue) in visible_dialogues.iter().enumerate() {
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
                                    RichText::new(dialogue.text.as_str())
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
    }
}
