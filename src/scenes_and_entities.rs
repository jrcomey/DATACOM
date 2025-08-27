use std::iter;

use cgmath::{Deg, Point3, Vector3, Quaternion, Matrix4};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    keyboard::PhysicalKey,
    window::Window,
};
use std::rc::Rc;
use std::cell::RefCell;
use log::error;
use cgmath::{EuclideanSpace, Rotation3};
use hdf5::{File, Selection};
use ndarray::s;

use crate::{model, camera};

use model::{DrawModel, Vertex};

#[derive(Copy, Clone)]
pub enum BehaviorType {
    EntityRotate,
    EntityTranslate,
    EntityChangePosition,
    ComponentRotate,
    ComponentTranslate,
    ComponentRotateConstantSpeed,
    ComponentChangeColor,
    Null,
}

impl BehaviorType {
    pub fn match_from_string(input_string: &str) -> BehaviorType {
        match input_string {
            "EntityRotate" => BehaviorType::EntityRotate,
            "EntityTranslate" => BehaviorType::EntityTranslate,
            "EntityChangePosition" => BehaviorType::EntityChangePosition,
            "ComponentRotate" => BehaviorType::ComponentRotate,
            "ComponentTranslate" => BehaviorType::ComponentTranslate,
            "ComponentRotateConstantSpeed" => BehaviorType::ComponentRotateConstantSpeed,
            "ComponentChangeColor" => BehaviorType::ComponentChangeColor,
            _ => BehaviorType::Null,
        }
    }
}

#[derive(Clone)]
pub struct Behavior {
    pub behavior_type: BehaviorType,
    pub data: Vec<f32>,
    pub data_counter: Option<usize>,
}

impl Behavior {
    pub fn new(behavior_type: BehaviorType, data: Vec<f32>, data_counter: Option<usize>) -> Behavior {
        Behavior {
            behavior_type: behavior_type,
            data: data,
            data_counter: data_counter,
        }
    }
    pub fn load_from_json(json: &serde_json::Value) -> Behavior {
        let data_temp: Vec<_> = json["data"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        let mut data: Vec<f32> = vec![];
        for data_point in data_temp.iter() {
            data.push(data_point.as_f64().unwrap() as f32);
        }

        let behavior_type: BehaviorType =
            BehaviorType::match_from_string(json["behaviorType"].as_str().unwrap());
        
        let data_counter = None;

        Behavior::new(behavior_type, data, data_counter)
    }

    pub fn load_from_hdf5(data: &ndarray::Array1<[f32; 12]>) -> hdf5::Result<Behavior> {
        let behavior_type = BehaviorType::EntityChangePosition;
        let a = 0;
        let b = 9;
        let data_vec: Vec<f32> = data
            .iter()
            .flat_map(|arrs| arrs[a..b].iter().cloned())
            .collect();
        let data_counter = Some(0);
        Ok(
            Behavior {
                behavior_type: behavior_type,
                data: data_vec,
                data_counter: data_counter,
            }
        )
    }
}

#[allow(dead_code)]
pub struct Entity {
    name: String,
    position: Rc<RefCell<Point3<f32>>>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
    models: Vec<model::Model>,
    behaviors: Vec<Behavior>,
}

impl Entity {
    pub fn load_from_json(json: &serde_json::Value, device: &wgpu::Device, model_bind_group_layout: &wgpu::BindGroupLayout) -> Entity {
        let name = json["Name"].to_string();

        // Position
        let position_temp = json["Position"]
            .as_array()
            .unwrap()
            .into_iter();
        let mut position_vec = Point3::<f32>::new(0.0, 0.0, 0.0);
        for (i, position) in position_temp.enumerate() {
            position_vec[i] = position.as_f64().unwrap() as f32;
        }

        // Rotation
        let rotation_temp = json["Rotation"]
            .as_array()
            .unwrap()
            .into_iter();
        let mut rotation_vec = Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, rotation_comp) in rotation_temp.enumerate() {
            rotation_vec[i] = rotation_comp.as_f64().unwrap() as f32;
        }

        // Scale
        let scale_temp = json["Scale"]
            .as_array()
            .unwrap()
            .into_iter();
        let mut scale_vec = Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, scale_comp) in scale_temp.enumerate() {
            scale_vec[i] = scale_comp.as_f64().unwrap() as f32;
        }

        let model_vec: Vec<_> = match json["Models"].as_array() {
            Some(array) => {
                let model_temp: Vec<_> = array.into_iter().collect();
                let mut model_vec = vec![];
                for i in model_temp.iter() {
                    model_vec.push(model::Model::load_from_json(*i, device, model_bind_group_layout));
                }
                model_vec
            }
            None => vec![],
        };

        let behavior_vec: Vec<_> = match json["Behaviors"].as_array() {
            Some(array) => {
                let behavior_temp: Vec<_> = array.into_iter().collect();
                let mut behavior_vec = vec![];
                for i in behavior_temp.iter() {
                    behavior_vec.push(Behavior::load_from_json(*i));
                }
                behavior_vec
            }
            None => vec![],
        };

        Entity {
            name: name,
            position: Rc::new(RefCell::new(position_vec)),
            rotation: Quaternion::from_sv(1.0, rotation_vec),
            scale: scale_vec,
            models: model_vec,
            behaviors: behavior_vec,
        }
    }

    pub fn load_from_hdf5(name: String, data: hdf5::Dataset, device: &wgpu::Device, model_bind_group_layout: &wgpu::BindGroupLayout) -> hdf5::Result<Entity> {
        // name
        println!("NAME: {}", name);

        // position
        let data_array: ndarray::Array1<[f32; 12]> = data.read()?;
        let initial_transform: [f32; 12] = data_array[0];
        let position = Point3::<f32>::new(initial_transform[0], initial_transform[1], initial_transform[2]);
        println!("POSITION: {:?}", position);

        // rotation
        let rotation = Vector3::<f32>::new(initial_transform[7], initial_transform[6], initial_transform[8]);
        println!("ROTATION: {:?}", rotation);

        // scale
        let mut scale = Vector3::<f32>::new(1.0, 1.0, 1.0);

        // model vec
        let mut name_root = name.clone();
        if let Some(val) = name_root.find("_"){
            name_root.truncate(val)
        }
        let name_root_str = name_root.as_str();
        println!("NAME STR: {}", name_root_str);
        let model_vec: Vec<_> = match name_root_str {
            "Blizzard" => {
                // scale = Vector3::<f32>::new(1.0, 1.0, 1.0);
                model::Model::load_from_json_file("data/object_loading/blizzard_initialize_full.json", device, model_bind_group_layout)
            }
            _ => vec![],
        };

        // behavior vec
        let behavior_vec: Vec<_> = match name_root_str {
            "Blizzard" => {
                // load entire data array into behavior and set type to SetPosition or similar
                vec![Behavior::load_from_hdf5(&data_array).unwrap()]
            }
            _ => vec![],
        };
        // println!("BEHAVIOR: {:?}", behavior_vec[0].data);

        // return entity
        Ok(
            Entity {
                name: name,
                position: Rc::new(RefCell::new(position)),
                rotation: Quaternion::from_sv(1.0, rotation),
                scale: scale,
                models: model_vec,
                behaviors: behavior_vec,
            }
        )
    }

    pub fn get_position(&self) -> Rc<RefCell<Point3<f32>>> { Rc::clone(&self.position) }

    pub fn set_position(&mut self, new_position: Point3<f32>) {
        *self.position.borrow_mut() = new_position;
        // println!("new position: ({}, {}, {})", new_position[0], new_position[1], new_position[2]);
    }

    fn to_matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::from_translation(self.position.borrow().to_vec());
        let rotation = Matrix4::from(self.rotation);
        let scale = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        // let rotation_correction = Matrix4::from_angle_x(Deg(-90.0));
        // rotation_correction * translation * rotation * scale
        translation * rotation * scale
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        queue: &wgpu::Queue,
    ) {
        let entity_matrix = self.to_matrix();
        for model in &self.models {
            // println!("drawing {}", model.name);
            let model_matrix = model.to_matrix();
            let full_transform = entity_matrix * model_matrix;
            let full_uniform: [[f32; 4]; 4] = full_transform.into();

            queue.write_buffer(
                &model.uniform_buffer,
                0,
                bytemuck::cast_slice(&[full_uniform]),
            );

            render_pass.draw_mesh(&model.obj, camera_bind_group, &model.bind_group);
        }
    }

    pub fn run_behavior(&mut self, behavior_index: usize) {
        // the borrow checker means that we have to refer to the behavior with self.behaviors[behavior_index] every time
        match self.behaviors[behavior_index].behavior_type {
            // Translate entity by vector
            BehaviorType::EntityTranslate => {
                let old_position = *self.position.borrow();
                let offset = Vector3::<f32>::new(self.behaviors[behavior_index].data[0], self.behaviors[behavior_index].data[1], self.behaviors[behavior_index].data[2]);
                self.set_position(old_position + offset);
            }

            // Change position to input
            BehaviorType::EntityChangePosition => {
                let counter = self.behaviors[behavior_index].data_counter.expect("Error in Entity::run_behavior : data counter is None");
                // println!("counter = {}", counter);

                let new_position = Point3::<f32>::new(self.behaviors[behavior_index].data[counter], self.behaviors[behavior_index].data[counter+1], self.behaviors[behavior_index].data[counter+2]);
                self.set_position(new_position);

                let rotation = Vector3::<f32>::new(self.behaviors[behavior_index].data[counter+6], self.behaviors[behavior_index].data[counter+7], self.behaviors[behavior_index].data[counter+8]);
                self.rotation = Quaternion::from_sv(1.0, rotation);

                // TODO: 16 is a magic number, referring to the milliseconds per timestep for the window refresh; figure out a better way to synchronize the timesteps
                self.behaviors[behavior_index].data_counter = Some(counter+9*16);
                // println!("data_counter = {}", self.behaviors[behavior_index].data_counter.unwrap());
                // println!("set position of entity {} to {:?} using counter {}", self.name, new_position, self.behaviors[behavior_index].data_counter.unwrap());
            }

            // Rotate item at constant speed
            BehaviorType::ComponentRotateConstantSpeed => {
                let model_id = self.behaviors[behavior_index].data[0] as u64;
                let rotation_factor = self.behaviors[behavior_index].data[1];
                let new_quaternion_vector = Vector3::<f32>::new(
                    (rotation_factor * self.behaviors[behavior_index].data[2]) as f32,
                    (rotation_factor * self.behaviors[behavior_index].data[4]) as f32,
                    (rotation_factor * self.behaviors[behavior_index].data[3]) as f32,
                );
                let new_quaternion = Quaternion::<f32>::from_sv(1.0, new_quaternion_vector);

                self.get_model(model_id).rotate(new_quaternion);
            }

            _ => return,
        }
    }

    pub fn run_behaviors(&mut self) {
        for i in 0..self.behaviors.len() {
            self.run_behavior(i);
        }
    }

    pub fn get_model(&mut self, model_component_id: u64) -> &mut model::Model {
        &mut self.models[model_component_id as usize]
    }
}

pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    lines_render_pipeline: wgpu::RenderPipeline,
    pub scene: Scene,
    projection: camera::Projection,
    pub camera_controller: camera::CameraController,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    // #[allow(dead_code)]
    // instances: Vec<Instance>,
    // #[allow(dead_code)]
    // instance_buffer: wgpu::Buffer, 
    window: &'a Window,
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
                polygon_mode: wgpu::PolygonMode::Line,
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let camera_roll = Quaternion::from_angle_z(Deg(-90.0));
        let camera_pitch = Quaternion::from_angle_y(Deg(0.0));
        let camera_yaw = Quaternion::from_angle_x(Deg(0.0));
        let camera_rotation = camera_roll * camera_pitch * camera_yaw;
        let camera = camera::Camera::new((-5.0, 0.0, 0.0), camera_rotation);
        let projection = camera::Projection::new(config.width, config.height, Deg(45.0), 0.1, 100.0);
        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);
        let camera_controller = camera::CameraController::new(8.0, 0.4, camera);


        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        let scene = if filepath.ends_with(".hdf5"){
            Scene::load_scene_from_hdf5(filepath, &device, &model_bind_group_layout).unwrap()
        } else {
            Scene::load_scene_from_json(filepath, &device, &model_bind_group_layout)
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
            )
        };
        
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            lines_render_pipeline,
            scene,
            projection,
            camera_controller,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            window,
            mouse_pressed: false,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
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
            } => self.camera_controller.process_keyboard(*key, *state, &self.scene.entities),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
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
            _ => false,
        }
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(dt);
        // log::info!("{:?}", self.camera);

        self.camera_uniform.update_view_proj(&self.camera_controller.camera(), &self.projection);
        log::info!("{:?}", self.camera_uniform);

        self.scene.run_behaviors();
        
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            
            render_pass.set_pipeline(&self.lines_render_pipeline);
            render_pass.draw_axes(&self.scene.axes, &self.camera_bind_group);

            render_pass.set_pipeline(&self.render_pipeline);
            // render_pass.draw_mesh_instanced(
            //     &self.obj_mesh,
            //     0..self.instances.len() as u32,
            //     &self.camera_bind_group,
            // );

            // we want a wgpu::Buffer derived from vertex_data
            // a Vec<[[f32; 4]; 4]>
            // each matrix contains entity_transform * model_transform
            //
            self.scene.draw(&mut render_pass, &self.camera_bind_group, &self.queue);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// Define the scene structure
pub struct Scene {
    pub axes: model::Axes,
    entities: Vec<Entity>,
}

impl Scene {
    // Method to add an entity to the scene
    // pub fn add_entity(&mut self, entity: Entity) {
    //     self.entities.push(entity);
    // }

    // Other scene-related methods...
    // pub fn new() -> Scene {
    //     Scene { entities: vec![] }
    // }

    // pub fn new_entities(entities: Vec<Entity>) -> Scene {
    //     Scene { entities: entities }
    // }

    // pub fn change_entity_position(&mut self, entity_id: u64, new_position: Point3<f64>) {
    //     self.entities[entity_id as usize].change_position(new_position);
    // }

    pub fn run_behaviors(&mut self) {
        for entity in &mut self.entities {
            entity.run_behaviors();
        }
    }

    // pub fn bhvr_msg_str(&mut self, json_unparsed: &str) {
    //     if json_unparsed.is_empty() {
    //         return;
    //     }
    //     // let json_parsed: Value = serde_json::from_str(json_unparsed);
    //     // self.cmd_msg(&json_parsed);

    //     let json_parsed: serde_json::Value = match serde_json::from_str(&json_unparsed) {
    //         serde_json::Result::Ok(val) => val,
    //         serde_json::Result::Err(_) => serde_json::Value::Null,
    //         // _ => {}
    //     };

    //     // debug!("Parsed JSON Packet: {}", json_parsed.to_string());

    //     if json_parsed != serde_json::Value::Null {
    //         for behavior in json_parsed.as_array().expect("").into_iter() {
    //             // debug!("Target ID: {}", cmd["targetEntityID"]);
    //             // debug!("Cmd Type: {}", cmd["commandType"]);
    //             // debug!("Data: {}", cmd["data"]);
    //             self.bhvr_msg(&behavior);
    //         }
    //         // self.bhvr_msg(&json_parsed);
    //     } else {
    //         error!("json failed to load!");
    //         error!("{}", json_unparsed);
    //     }
    // }

    // pub fn bhvr_msg(&mut self, json_parsed: &serde_json::Value) {

    //     // debug!("Target ID: {}", json_parsed["targetEntityID"]);
    //     let target_entity_id = json_parsed["targetEntityID"].as_u64().unwrap() as usize;

    //     let behavior = Behavior::load_from_json(json_parsed);

    //     self.get_entity(target_entity_id).expect("Out of bounds!").run_behavior(behavior);
    // }

    fn load_scene_from_hdf5(filepath: &str, device: &wgpu::Device, model_bind_group_layout: &wgpu::BindGroupLayout) -> hdf5::Result<Scene> {
        let file = File::open(filepath).unwrap();
        let vehicles = file.group("Vehicles").unwrap();
        let vehicles_vec = vehicles.groups().unwrap();
        let mut entity_vec = vec![];
        for vehicle in vehicles_vec.iter() {
            let name_full = vehicle.name();
            let name = name_full["/Vehicles/".len()..].to_string();
            let data = vehicle.dataset("states").unwrap();
            entity_vec.push(Entity::load_from_hdf5(name, data, device, model_bind_group_layout).unwrap());
        }
        println!("LOADED {} ENTITIES INTO SCENE", entity_vec.len());

        let axes = model::Axes::new(device);

        Ok(Scene {
            axes: axes,
            entities: entity_vec,
        })
    }

    fn load_scene_from_json(filepath: &str, device: &wgpu::Device, model_bind_group_layout: &wgpu::BindGroupLayout) -> Scene {
        let json_unparsed = std::fs::read_to_string(filepath).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_unparsed).unwrap();

        let entity_temp: Vec<_> = json["entities"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        let mut entity_vec = vec![];
        for i in entity_temp.iter() {
            entity_vec.push(Entity::load_from_json(*i, device, model_bind_group_layout));
        }

        let axes = model::Axes::new(device);

        Scene{
            axes: axes,
            entities: entity_vec,
        }
    }

    fn get_entity(&mut self, entity_id: usize) -> Option<&mut Entity> {
        self.entities.get_mut(entity_id)
    }

    // pub fn load_from_network(addr: &str) -> Result<Scene, Error> {
    //     // Open port
    //     let listener = TcpListener::bind(addr).unwrap();
    //     let mut num_attempt = 0;
        
    //     // Attempt to recieve initialization packet and parse when successful.
    //     let initialization_packet = loop {
    //         match listener.accept() {
    //             Ok((stream, _)) => {
    //                 // debug!("{}", com::from_network(&stream));
    //                 break com::from_network(&stream)
    //             },
    //             _ => {
    //                 num_attempt += 1;
    //                 debug!("No packet recieved. Trying attempt {}...", num_attempt);
    //                 std::thread::sleep(Duration::from_millis(100));
    //             },
    //         }
    //     };
    //     info!("Received initialization file");
    //     // debug!("Initialization file: {}", initialization_packet);

    //     // Receive and save model files
    //     for stream in listener.incoming() {

    //         let mut local_stream = stream.unwrap();
    //         match com::from_network_with_protocol(&mut local_stream) {
    //             Ok(_) => {},
    //             Err("END") => {
    //                 debug!("Finished recieving files!");
    //             }
    //             _ => {break}
    //         }
    //     }

    //     info!("All files recieved.");

    //     //
        
    //     // Load Scene from initialization packet

    //     Ok(Scene::load_from_json_str(&initialization_packet))
        
    // }

    /// Clear all entities 
    // pub fn clear_all(&mut self) {
    //     self.entities = vec![];
    // }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        queue: &wgpu::Queue,
    ){
        // render_pass.draw_axes(&self.axes, camera_bind_group);

        for entity in &self.entities {
            entity.draw(render_pass, camera_bind_group, queue);
        }
    }
}