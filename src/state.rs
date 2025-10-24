use std::iter;

use winit::{
    event::*,
    keyboard::PhysicalKey,
    window::Window,
};
use wgpu::{util::DeviceExt, TextureUsages};
use cgmath::{Deg, Quaternion, Matrix4, Rotation3};

use crate::scenes_and_entities::Scene;
use crate::model;
use crate::camera;
use crate::text::GlyphVertex;

use model::{Vertex, DrawModel};

pub struct Viewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    pub camera_controller: camera::CameraController,
    projection: camera::Projection,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    border_color: cgmath::Vector3<f32>,
}

impl Viewport {
    fn new(
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        camera: camera::Camera, 
        device: &wgpu::Device, 
        camera_bind_group_layout: &wgpu::BindGroupLayout, 
        border_color: cgmath::Vector3<f32>, 
    ) -> Self {
        let projection = camera::Projection::new(w as u32, h as u32, Deg(45.0), 0.1, 100.0);
        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);
        let camera_controller = camera::CameraController::new(8.0, 0.4, camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        Viewport {
            x,
            y,
            width: w,
            height: h,
            camera_controller,
            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            border_color,
        }
    }
}

pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    offscreen_texture: wgpu::Texture,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    lines_render_pipeline: wgpu::RenderPipeline,
    text_render_pipeline: wgpu::RenderPipeline,
    scene: Scene,
    pub viewports: Vec<Viewport>,
    ortho_transform_matrix: cgmath::Matrix4<f32>,
    ortho_transform_buffer: wgpu::Buffer,
    ortho_matrix_bind_group: wgpu::BindGroup,
    window: &'a Window,
    pub framerate: f32,
    pub mouse_pressed: bool,
}

impl<'a> State<'a> {
    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: wgpu::ShaderModuleDescriptor,
        topology: wgpu::PrimitiveTopology,
        polygon_mode: wgpu::PolygonMode,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(shader);

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{:?}", shader)),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_layouts,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: polygon_mode,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            cache: None,
        })
    }

    pub async fn new(window: &'a Window, filepath: &str) -> State<'a> {
        let size = window.inner_size();
        // println!("window size: {} * {} = {}", size.width, size.height, size.width * size.height);

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        log::warn!("WGPU setup");
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        log::warn!("device and queue");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::POLYGON_MODE_LINE,
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                    trace: wgpu::Trace::Off, // Trace path
                },
            )
            .await
            .unwrap();
        assert!(device.features().contains(wgpu::Features::POLYGON_MODE_LINE), "Wireframe polygon mode not supported!");

        log::warn!("Surface");
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let offscreen_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Target"),
            size: wgpu::Extent3d {
                width: size.width, 
                height: size.height, 
                depth_or_array_layers: 1
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let camera_yaw_front = Quaternion::from_angle_z(Deg(-90.0));
        let camera_yaw_side = Quaternion::from_angle_z(Deg(0.0));
        let camera_roll = Quaternion::from_angle_y(Deg(0.0));
        let camera_pitch = Quaternion::from_angle_x(Deg(0.0));
        let camera_rotation_front = camera_yaw_front * camera_roll * camera_pitch;
        let camera_rotation_side = camera_yaw_side * camera_roll * camera_pitch;
        let camera_front = camera::Camera::new((-5.0, 0.0, 0.0), camera_rotation_front);
        let camera_side = camera::Camera::new((0.0, -5.0, 0.0), camera_rotation_side);

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Camera Bind Group Layout"),
            });

        let viewports = vec![
            Viewport::new(0.0, 0.0, (size.width/2) as f32, size.height as f32, camera_front, &device, &camera_bind_group_layout, cgmath::Vector3::<f32>::new(0.0, 255.0, 0.0)),
            Viewport::new((size.width/2) as f32, 0.0, (size.width/2) as f32, size.height as f32, camera_side, &device, &camera_bind_group_layout, cgmath::Vector3::<f32>::new(0.0, 0.0, 255.0)),
        ];

        let model_bind_group_layout = 
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Model Bind Group Layout"),
        });

        let ortho_transform_matrix: Matrix4<f32> = cgmath::ortho(0.0, size.width as f32, size.height as f32, 0.0, -1.0, 1.0);
        let ortho_transform_arr: [[f32; 4]; 4] = ortho_transform_matrix.into();

        let ortho_transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ortho transform matrix buffer"),
            contents: bytemuck::cast_slice(&ortho_transform_arr),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let ortho_matrix_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Ortho Transformation Matrix"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ]
        });

        let ortho_matrix_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ortho Matrix Bind Group"),
            layout: &ortho_matrix_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ortho_transform_buffer.as_entire_binding(),
            }],
        });

        let text_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("Text Bind Group Layout"),
        });

        let scene = if filepath.ends_with(".hdf5"){
            Scene::load_scene_from_hdf5(
                filepath, 
                &device, 
                &queue, 
                &config.format, 
                &model_bind_group_layout, 
                &text_bind_group_layout, 
                size.width, 
                size.height, 
            ).unwrap()
        } else if filepath.ends_with(".json"){
            Scene::load_scene_from_json(
                filepath, 
                &device, 
                &queue, 
                &config.format, 
                &model_bind_group_layout, 
                &text_bind_group_layout, 
                size.width, 
                size.height, 
            )
        } else {
            Scene::load_scene_from_network(
                filepath, 
                &device, 
                &queue,
                &config.format,
                &model_bind_group_layout, 
                &text_bind_group_layout, 
                size.width, 
                size.height, 
            ).unwrap()
        };

        let render_pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &model_bind_group_layout,
                    ],
                push_constant_ranges: &[],
            });

        let text_render_pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &ortho_matrix_bind_group_layout,
                    &text_bind_group_layout,
                    ],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
            };
            State::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                &[model::ModelVertex::desc()],
                shader,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::PolygonMode::Line,
            )
        };

        let lines_render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Lines Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
            };
            State::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                &[model::ModelVertex::desc()],
                shader,
                wgpu::PrimitiveTopology::LineList,
                wgpu::PolygonMode::Line,
            )
        };

        let text_render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Text Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text_shader.wgsl").into()),
            };
            State::create_render_pipeline(
                &device, 
                &text_render_pipeline_layout, 
                config.format, 
                &[GlyphVertex::desc()], 
                shader, 
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::PolygonMode::Fill,
            )
        };
        
        surface.configure(&device, &config);

        Self {
            surface,
            offscreen_texture,
            device,
            queue,
            config,
            size,
            render_pipeline,
            lines_render_pipeline,
            text_render_pipeline,
            scene,
            viewports,
            ortho_transform_matrix,
            ortho_transform_buffer,
            ortho_matrix_bind_group,
            window,
            framerate: 60.0,
            mouse_pressed: false,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            for viewport in &mut self.viewports {
                viewport.projection.resize(new_size.width, new_size.height);
            }
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            let ortho_transform_arr: [[f32; 4]; 4] = self.ortho_transform_matrix.into();
            self.queue.write_buffer(
                &self.ortho_transform_buffer, 
                0, 
                bytemuck::cast_slice(&ortho_transform_arr)
            );
            self.surface.configure(&self.device, &self.config);
        }
    }
    
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.viewports[0].camera_controller.process_keyboard(*key, *state, &self.scene.entities),
            WindowEvent::MouseWheel { delta, .. } => {
                self.viewports[0].camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved {
                position,
                ..
            } if self.mouse_pressed => {
                // println!("mouse pressed");
                // println!("({}, {})", position.x, position.y);
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: std::time::Duration, should_save_to_file: bool) {
        for viewport in &mut self.viewports {
            viewport.camera_controller.update_camera(dt);
            viewport.camera_uniform.update_view_proj(&viewport.camera_controller.camera(), &viewport.projection);
            log::info!("{:?}", viewport.camera_uniform);
        
            self.queue.write_buffer(
                &viewport.camera_buffer,
                0,
                bytemuck::cast_slice(&[viewport.camera_uniform]),
            );
        }
        self.framerate = dt.as_secs_f32().recip();
        let fr_str = format!("{:.1} fps", self.framerate);
        self.scene.text_boxes[0].change_text(&self.device, fr_str);


        self.scene.run_behaviors();

        if should_save_to_file {
            self.scene.read_and_write_capture_buffers(
                &self.device,
                &self.queue,
                &self.offscreen_texture,
                self.size.width,
                self.size.height
            );
        }
    }

    pub fn render(&mut self, should_save_to_file: bool) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let offscreen_view = self.offscreen_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let target = if should_save_to_file {
            offscreen_view
        } else {
            view
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            for viewport in self.viewports.iter() {
                render_pass.set_viewport(viewport.x, viewport.y, viewport.width, viewport.height, 0.0, 1.0);

                render_pass.set_pipeline(&self.lines_render_pipeline);
                render_pass.draw_axes(&self.scene.axes, &viewport.camera_bind_group);

                render_pass.set_pipeline(&self.render_pipeline);
                self.scene.draw(&mut render_pass, &viewport.camera_bind_group, &self.ortho_matrix_bind_group, &self.text_render_pipeline, &self.queue);
            }
        }

        if should_save_to_file {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.offscreen_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
            wgpu::TexelCopyTextureInfo {
                    texture: &output.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
            wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}