use std::{collections::HashMap, mem::size_of, path::Path};

use crate::game_object::{GameObject2D, RenderLayer};
use image::{DynamicImage, GenericImageView};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 4],
    tex_coord: [f32; 2],
}

// SAFETY: Vertex is repr(C) with only Copy types.
unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

fn create_vertices() -> ([Vertex; 4], [u16; 6]) {
    let vertex_data = [
        vertex([1, 1, 0], [1, 0]),
        vertex([-1, 1, 0], [0, 0]),
        vertex([-1, -1, 0], [0, 1]),
        vertex([1, -1, 0], [1, 1]),
    ];
    let index_data = [0, 1, 2, 2, 3, 0];
    (vertex_data, index_data)
}

struct RenderObject {
    game_object: GameObject2D,
    order: u64,
    diffuse_bind_group: wgpu::BindGroup,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
}

pub struct Tex {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    pipeline_wire: Option<wgpu::RenderPipeline>,
    view_proj: glam::Mat4,
    objects: Vec<RenderObject>,
    object_lookup: HashMap<String, usize>,
    next_object_order: u64,
}

impl Tex {
    fn build_view_projection(aspect_ratio: f32) -> glam::Mat4 {
        let projection = glam::Mat4::orthographic_rh(
            -2.0 * aspect_ratio,
            2.0 * aspect_ratio,
            -2.0,
            2.0,
            0.1,
            10.0,
        );
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(0.0, 0.0, 5.0),
            glam::Vec3::ZERO,
            glam::Vec3::Y,
        );
        projection * view
    }

    fn build_model_view_projection(view_proj: glam::Mat4, object: &GameObject2D) -> glam::Mat4 {
        let model =
            glam::Mat4::from_translation(glam::Vec3::new(
                object.position.x,
                object.position.y,
                0.0,
            )) * glam::Mat4::from_scale(glam::Vec3::new(object.scale.x, object.scale.y, 1.0));
        view_proj * model
    }

    fn create_uniform_resources(
        device: &wgpu::Device,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
        transform: glam::Mat4,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let matrix = transform.to_cols_array();
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("game_object_uniform"),
            contents: bytemuck::bytes_of(&matrix),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
            label: Some("game_object_uniform_bind_group"),
        });
        (uniform_buf, uniform_bind_group)
    }

    fn create_diffuse_bind_group_from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        diffuse_image: DynamicImage,
        label: &str,
    ) -> wgpu::BindGroup {
        let diffuse_rgba = diffuse_image.to_rgba8();
        let dimensions = diffuse_image.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture_label = format!("{label}_texture");
        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(texture_label.as_str()),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &diffuse_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let diffuse_texture_view =
            diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_label = format!("{label}_bind_group");
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some(bind_group_label.as_str()),
        })
    }

    fn sort_objects(&mut self) {
        self.objects.sort_by_key(|object| {
            let (layer_order, z_index) = object.game_object.render_sort_key();
            (layer_order, z_index, object.order)
        });
        self.rebuild_object_lookup();
    }

    fn rebuild_object_lookup(&mut self) {
        self.object_lookup.clear();
        for (index, object) in self.objects.iter().enumerate() {
            self.object_lookup
                .insert(object.game_object.scene_key(), index);
        }
    }

    fn push_game_object_from_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        game_object: GameObject2D,
        diffuse_image: DynamicImage,
    ) {
        let texture_label = if game_object.texture_path.is_empty() {
            "scene_object".to_string()
        } else {
            game_object.texture_path.clone()
        };

        let diffuse_bind_group = Self::create_diffuse_bind_group_from_image(
            device,
            queue,
            &self.texture_bind_group_layout,
            diffuse_image,
            texture_label.as_str(),
        );

        let transform = Self::build_model_view_projection(self.view_proj, &game_object);
        let (uniform_buf, uniform_bind_group) =
            Self::create_uniform_resources(device, &self.uniform_bind_group_layout, transform);

        let object = RenderObject {
            game_object,
            order: self.next_object_order,
            diffuse_bind_group,
            uniform_bind_group,
            uniform_buf,
        };
        self.next_object_order = self.next_object_order.saturating_add(1);

        self.objects.push(object);
        self.sort_objects();
    }

    pub fn init(
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {
        let (vertex_data, index_data) = create_vertices();

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 4,
                    shader_location: 1,
                },
            ],
        }];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("main_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let pipeline_wire = if device
            .features()
            .contains(wgpu::Features::POLYGON_MODE_LINE)
        {
            Some(
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("wire_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &vertex_buffers,
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_wire"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: config.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    operation: wgpu::BlendOperation::Add,
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                },
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::Line,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            )
        } else {
            None
        };

        let tex = Self {
            vertex_buf,
            index_buf,
            index_count: index_data.len() as u32,
            texture_bind_group_layout,
            uniform_bind_group_layout,
            pipeline,
            pipeline_wire,
            view_proj: Self::build_view_projection(config.width as f32 / config.height as f32),
            objects: Vec::new(),
            object_lookup: HashMap::new(),
            next_object_order: 0,
        };

        println!("done!");
        tex
    }

    #[allow(dead_code)]
    pub fn create_game_object(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pos: [f32; 2],
        scale: [f32; 2],
        texture: &str,
    ) -> Result<(), String> {
        self.create_game_object_layered(
            device,
            queue,
            pos,
            scale,
            texture,
            RenderLayer::Character,
            0,
        )
    }

    pub fn create_game_object_layered(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pos: [f32; 2],
        scale: [f32; 2],
        texture: &str,
        layer: RenderLayer,
        z_index: i32,
    ) -> Result<(), String> {
        self.create_game_object_from_definition(
            device,
            queue,
            GameObject2D::new(pos, scale, texture, layer, z_index),
        )
    }

    pub fn create_game_object_from_definition(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        object: GameObject2D,
    ) -> Result<(), String> {
        let diffuse_image = image::open(Path::new(&object.texture_path)).map_err(|err| {
            format!(
                "failed to load texture '{}': {err}",
                object.texture_path.as_str()
            )
        })?;
        self.push_game_object_from_image(device, queue, object, diffuse_image);
        Ok(())
    }

    pub fn apply_game_object_from_definition(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        object: GameObject2D,
    ) -> Result<(), String> {
        let object_key = object.scene_key();
        if let Some(index) = self.object_lookup.get(&object_key).copied() {
            self.update_existing_object(index, device, queue, object)?;
            return Ok(());
        }

        self.create_game_object_from_definition(device, queue, object)
    }

    fn update_existing_object(
        &mut self,
        index: usize,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        object: GameObject2D,
    ) -> Result<(), String> {
        let new_matrix = Self::build_model_view_projection(self.view_proj, &object).to_cols_array();

        let (order_changed, texture_changed, texture_path_for_reload) = {
            let existing = self
                .objects
                .get_mut(index)
                .ok_or_else(|| format!("invalid object index {index}"))?;

            let order_changed = existing.game_object.render_sort_key() != object.render_sort_key();
            let texture_changed = existing.game_object.texture_path != object.texture_path;
            let texture_path_for_reload = if texture_changed {
                Some(object.texture_path.clone())
            } else {
                None
            };

            existing.game_object = object;
            queue.write_buffer(&existing.uniform_buf, 0, bytemuck::bytes_of(&new_matrix));

            (order_changed, texture_changed, texture_path_for_reload)
        };

        if texture_changed {
            let texture_path = texture_path_for_reload.expect("texture_changed checked above");
            let diffuse_image = image::open(Path::new(&texture_path))
                .map_err(|err| format!("failed to load texture '{texture_path}': {err}"))?;
            let new_bind_group = Self::create_diffuse_bind_group_from_image(
                device,
                queue,
                &self.texture_bind_group_layout,
                diffuse_image,
                texture_path.as_str(),
            );
            if let Some(existing) = self.objects.get_mut(index) {
                existing.diffuse_bind_group = new_bind_group;
            }
        }

        if order_changed {
            self.sort_objects();
        } else {
            self.rebuild_object_lookup();
        }

        Ok(())
    }

    pub fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.view_proj = Self::build_view_projection(config.width as f32 / config.height as f32);

        for object in &self.objects {
            let matrix = Self::build_model_view_projection(self.view_proj, &object.game_object)
                .to_cols_array();
            queue.write_buffer(&object.uniform_buf, 0, bytemuck::bytes_of(&matrix));
        }
    }

    pub fn render(&mut self, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
            rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));

            for object in &self.objects {
                if object.game_object.hidden {
                    continue;
                }

                rpass.set_pipeline(&self.pipeline);
                rpass.set_bind_group(0, &object.diffuse_bind_group, &[]);
                rpass.set_bind_group(1, &object.uniform_bind_group, &[]);
                rpass.draw_indexed(0..self.index_count, 0, 0..1);

                if let Some(ref pipe) = self.pipeline_wire {
                    rpass.set_pipeline(pipe);
                    rpass.set_bind_group(0, &object.diffuse_bind_group, &[]);
                    rpass.set_bind_group(1, &object.uniform_bind_group, &[]);
                    rpass.draw_indexed(0..self.index_count, 0, 0..1);
                }
            }
        }

        queue.submit(Some(encoder.finish()));
    }
}
