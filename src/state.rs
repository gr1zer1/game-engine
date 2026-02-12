use std::sync::Arc;
use wgpu::{Adapter, Instance, Surface};
use winit::window::Window;

#[allow(unused)]
pub struct State {
    pub window: Arc<Window>,
    pub instance: Instance,
    pub surface: Surface<'static>,
    pub adapter: Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub render_pipeline: Option<wgpu::RenderPipeline>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self, ()> {
        let instance = Instance::default();

        let surface = unsafe {
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(
                instance.create_surface(window.as_ref()).unwrap(),
            )
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::default(),
            })
            .await
            .unwrap();

        Ok(Self {
            window,
            instance,
            surface,
            adapter,
            device,
            queue,
            config: None,
            render_pipeline: None,
        })
    }

    pub fn redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resumed(&mut self) {
        let config = self
            .surface
            .get_default_config(
                &self.adapter,
                self.window.inner_size().width,
                self.window.inner_size().height,
            )
            .unwrap();
        self.config = Some(config);

        self.surface
            .configure(&self.device, &self.config.as_ref().unwrap());
    }
}
