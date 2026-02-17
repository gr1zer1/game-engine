use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

mod state;
use state::State;
mod game_object;
mod scene_objects;
mod tex;
use tex::Tex;

struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    tex: Option<tex::Tex>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            state: None,
            tex: None,
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

            for object in scene_objects::read_initial_scene_objects() {
                tex.create_game_object_layered(
                    &state.device,
                    &state.queue,
                    object.position.to_array(),
                    object.scale.to_array(),
                    &object.texture_path,
                    object.layer,
                    object.z_index,
                )
                .expect("failed to create scene object");
            }

            self.tex = Some(tex);
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
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(tex)) = (self.state.as_ref(), self.tex.as_mut()) {
                    // Получаем текущий кадр из поверхности окна
                    let frame = state
                        .surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");

                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    // Рендерим куб в этот кадр
                    tex.render(&view, &state.device, &state.queue);

                    // Показываем кадр на экране
                    frame.present();
                }

                // let frame = self.state.as_ref().unwrap().surface
                //     .get_current_texture()
                //     .expect("Failed to acquire next swap chain texture");
                // let view = frame
                //     .texture
                //     .create_view(&wgpu::TextureViewDescriptor::default());
                // let mut encoder =
                //     self.state.as_ref().unwrap().device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                //         label: None,
                //     });
                // {
                //     let mut rpass =
                //         encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                //             label: None,
                //             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                //                 view: &view,
                //                 depth_slice: None,
                //                 resolve_target: None,
                //                 ops: wgpu::Operations {
                //                     load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                //                     store: wgpu::StoreOp::Store,
                //                 },
                //             })],
                //             depth_stencil_attachment: None,
                //             timestamp_writes: None,
                //             occlusion_query_set: None,
                //             multiview_mask: None,
                //         });
                //     rpass.set_pipeline(&self.state.as_mut().unwrap().render_pipeline.as_mut().unwrap());
                //     rpass.draw(0..3, 0..1);
                // }

                // self.state.as_ref().unwrap().queue.submit(Some(encoder.finish()));
                // self.window.as_ref().unwrap().pre_present_notify();
                // frame.present();
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
