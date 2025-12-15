
use cgmath::{Point3, Vector3, Quaternion, Matrix4};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::path::Path;
use std::io::{Read, Seek, Write};
use log::{debug, info, error};
use cgmath::EuclideanSpace;
use std::process::{Command, Stdio};
use ndarray::{ArrayBase, OwnedRepr, Dim};

use crate::{model, com, text};

use model::DrawModel;
use text::{TextDisplay};

const DATA_ARR_WIDTH: usize = 9;
const AVERAGE_REFRESH_RATE: usize = 16;
const NUM_CAPTURE_BUFFERS: usize = 3;
const BYTES_PER_PIXEL: u32 = 4;
const CHUNK_LENGTH: u64 = 1024;

#[derive(Copy, Clone)]
pub enum BehaviorType {
    EntityRotate,
    EntityTranslate,
    EntityChangeTransform,
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
            "EntityChangeTransform" => BehaviorType::EntityChangeTransform,
            "ComponentRotate" => BehaviorType::ComponentRotate,
            "ComponentTranslate" => BehaviorType::ComponentTranslate,
            "ComponentRotateConstantSpeed" => BehaviorType::ComponentRotateConstantSpeed,
            "ComponentChangeColor" => BehaviorType::ComponentChangeColor,
            _ => BehaviorType::Null,
        }
    }

    fn is_constant_behavior(behavior_type: BehaviorType) -> bool {
        match behavior_type {
            BehaviorType::EntityTranslate => true,
            BehaviorType::ComponentRotateConstantSpeed => true,
            _ => false,
        }
    }
}

pub struct Behavior {
    pub behavior_type: BehaviorType,
    pub data: Vec<f32>,
    pub is_constant_behavior: bool,
    data_file_path: Option<String>,
}

impl Behavior {
    pub fn new(behavior_type: BehaviorType, data: Vec<f32>, data_file_path: Option<String>) -> Behavior {
        let is_constant_behavior = BehaviorType::is_constant_behavior(behavior_type);
        Behavior {
            behavior_type,
            data,
            is_constant_behavior,
            data_file_path, 
        }
    }
    pub fn load_from_json(json: &serde_json::Value) -> Behavior {
        let behavior_type: BehaviorType =
            BehaviorType::match_from_string(json["behaviorType"].as_str().unwrap());
        let mut data_temp: Vec<_> = json["data"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        let mut data: Vec<f32> = vec![];
        let data_file_path = if !BehaviorType::is_constant_behavior(behavior_type) {
            Some(
                data_temp.remove(0)
                .to_string()
            )
        } else {
            None
        };

        for data_point in data_temp.iter() {
            data.push(data_point.as_f64().unwrap() as f32);
        }

        Behavior::new(behavior_type, data, data_file_path)
    }

    pub fn load_from_hdf5(data: &ArrayBase<OwnedRepr<[f32; 12]>, Dim<[usize; 1]>>) -> hdf5::Result<Behavior> {
        let behavior_type = BehaviorType::EntityChangeTransform;
        let a = 0;
        let b = DATA_ARR_WIDTH;
        let data_vec: Vec<f32> = data
            .iter()
            .flat_map(|arrs| arrs[a..b].iter().cloned())
            .collect();

        Ok(Behavior::new(behavior_type, data_vec, None))
    }

    // fn retrieve_data
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
        let data_array: ArrayBase<OwnedRepr<[f32; 12]>, Dim<[usize; 1]>>  = data.read()?;
        let initial_transform: [f32; 12] = data_array[0];
        let position = Point3::<f32>::new(initial_transform[0], initial_transform[1], initial_transform[2]);
        println!("POSITION: {:?}", position);

        // rotation
        let rotation = Vector3::<f32>::new(initial_transform[7], initial_transform[6], initial_transform[8]);
        println!("ROTATION: {:?}", rotation);

        // scale
        let scale = Vector3::<f32>::new(1.0, 1.0, 1.0);

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

    // pub fn load_from_network() -> Entity {

    // }

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
    
    fn run_behavior_new(&mut self){
        // we need to borrow self mutably so we can change position
        let index = 0usize;
        let pos = Point3::<f32>::new(0.0, 0.0, 0.0);
        let data = &self.behaviors[index].data;
        self.set_position(pos);

        let path_ref = &self.behaviors[index].data_file_path;
        if let Some(path) = path_ref {
            Self::retrieve_data_chunk(path);
        }
    }

    pub fn run_behavior(&mut self, behavior_index: usize, data_counter: Option<usize>) {
        // the borrow checker means that we have to refer to the behavior with self.behaviors[behavior_index] every time
        let behavior = &self.behaviors[behavior_index];
        let behavior_type = behavior.behavior_type;
        let data = &self.behaviors[behavior_index].data;

        match behavior_type {
            // Translate entity by vector
            BehaviorType::EntityTranslate => {
                let old_position = *self.position.borrow();
                let offset = Vector3::<f32>::new(data[0], data[1], data[2]);
                self.set_position(old_position + offset);
            }

            // Change position to input
            BehaviorType::EntityChangeTransform => {
                let counter = data_counter.expect("Error in Entity::run_behavior : data counter is None");
                // println!("counter = {}", counter);

                let new_position = Point3::<f32>::new(data[counter], data[counter+1], data[counter+2]);
                let rotation = Vector3::<f32>::new(data[counter+3], data[counter+4], data[counter+5]);
                self.set_position(new_position);
                self.rotation = Quaternion::from_sv(1.0, rotation);

                if counter+6 >= self.behaviors[behavior_index].data.len() / 2 {
                    let path_ref = &self.behaviors[behavior_index].data_file_path;
                    if let Some(path) = path_ref {
                        Self::retrieve_data_chunk(path);
                    }
                    // Self::retrieve_data_chunk(.unwrap());
                }

                // TODO: 16 is a magic number, referring to the milliseconds per timestep for the window refresh; figure out a better way to synchronize the timesteps
                // self.behaviors[behavior_index].data_counter = Some(counter+9*16);
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

    pub fn run_behaviors(&mut self, data_counter: Option<usize>) {
        for i in 0..self.behaviors.len() {
            self.run_behavior(i, data_counter);
        }
    }

    fn retrieve_data_chunk(target_file_path_str: &String){
        let target_file_path = Path::new(target_file_path_str);
        let mut file = std::fs::File::create(target_file_path).unwrap();
        let mut buffer = [0; CHUNK_LENGTH as usize];
        file.read(&mut buffer).unwrap();
        
        // delete the chunk from the file
        let temp_path = target_file_path.with_extension("tmp");
        let mut temp_file = std::fs::File::create(&temp_path).unwrap();

        file.seek(std::io::SeekFrom::Start(CHUNK_LENGTH));
        std::io::copy(&mut file, &mut temp_file);
        temp_file.sync_all();
        std::fs::rename(&temp_path, target_file_path);
    }

    pub fn get_model(&mut self, model_component_id: u64) -> &mut model::Model {
        &mut self.models[model_component_id as usize]
    }
}

// Define the scene structure
pub struct Scene {
    pub axes: model::Axes,
    pub entities: Vec<Entity>,
    pub terrain: model::Terrain,
    pub text_boxes: Vec<text::TextDisplay>,
    timesteps: Option<usize>,
    data_counter: Option<usize>,
    frame_counter: usize,
    capture_buffers: Vec<wgpu::Buffer>,
    screen_recordings: Vec<Vec<u8>>,
}

impl Scene {
    pub fn new(
        entities: Vec<Entity>, 
        timesteps: Option<usize>, 
        data_counter: Option<usize>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat,
        text_bind_group_layout: &wgpu::BindGroupLayout,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        let axes = model::Axes::new(device);
        let text_boxes = Scene::init_text_boxes(device, queue, format, text_bind_group_layout, 60.0);
        let frame_counter: usize = 0;
        let capture_buffers = Scene::init_capture_buffers(
            device, 
            NUM_CAPTURE_BUFFERS, 
            (Into::<u64>::into(BYTES_PER_PIXEL) * ((screen_width * screen_height) as u64)) as wgpu::BufferAddress
        );

        let terrain = model::Terrain::new(cgmath::Vector3::<f32>::new(255.0, 0.0, 0.0), device);

        let screen_recordings = Vec::new();
        Scene {
            axes,
            entities,
            terrain,
            text_boxes,
            timesteps,
            data_counter,
            frame_counter,
            capture_buffers,
            screen_recordings,
        }
    }

    fn init_text_boxes(
        device: &wgpu::Device, 
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat,
        text_bind_group_layout: &wgpu::BindGroupLayout, 
        framerate: f32,
    ) -> Vec<TextDisplay> {
        let (image_atlas, glyph_map) = text::load_font_atlas(&text::get_font(), 100.0);
        let glyph_map = Arc::new(glyph_map);
        let texture_atlas = Arc::new(text::create_texture_atlas(device, queue, format, image_atlas));
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let text_objects: Vec<TextDisplay> = vec![
            TextDisplay::new(
                framerate.to_string(), 
                glyph_map.clone(), 
                30.0, 
                100.0, 
                cgmath::Vector3::new(0.0, 255.0, 0.0),
                device,
                &texture_atlas,
                &atlas_sampler,
                text_bind_group_layout,
            )
        ];

        // TextDisplay::new("Hello World!".to_string(), glyph_map.clone(), texture_atlas.clone(), 0.0, 0.0, green_vec()),
        // TextDisplay::new("DATACOM VER 0.1.0".to_string(), glyph_map.clone(), texture_atlas.clone(), -1.0, 0.90, green_vec()),
        // TextDisplay::new((' '..='~').collect(), glyph_map.clone(), texture_atlas.clone(), -1.0, -1.0, green_vec()),
        // TextDisplay::new("FPS Counter: 0.0".to_string(), glyph_map.clone(), texture_atlas.clone(), 0.6, 0.9, cyan_vec()),

        text_objects
    }

    pub fn run_behaviors(&mut self) {
        for entity in &mut self.entities {
            entity.run_behaviors(self.data_counter);
            if let Some(c) = self.data_counter {
                self.data_counter = Some(c + DATA_ARR_WIDTH * AVERAGE_REFRESH_RATE);
            }
        }
    }

    pub fn increment_frame_counter(&mut self){
        self.frame_counter += 1;
        // println!("frame {}", self.frame_counter);
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

    pub fn load_scene(
        filepath: &str, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat, 
        model_bind_group_layout: &wgpu::BindGroupLayout, 
        text_bind_group_layout: &wgpu::BindGroupLayout,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        if filepath.ends_with(".hdf5"){
            Scene::load_scene_from_hdf5(
                filepath, 
                device, 
                queue, 
                format, 
                model_bind_group_layout, 
                text_bind_group_layout, 
                screen_width, 
                screen_height, 
            ).unwrap()
        } else if filepath.ends_with(".json"){
            Scene::load_scene_from_json(
                filepath, 
                device, 
                queue, 
                format, 
                model_bind_group_layout, 
                text_bind_group_layout, 
                screen_width, 
                screen_height, 
            )
        } else {
            Scene::load_scene_from_json(
                filepath, 
                device, 
                queue, 
                format, 
                model_bind_group_layout, 
                text_bind_group_layout, 
                screen_width, 
                screen_height, 
            )
            // Scene::load_scene_from_network(
            //     filepath, 
            //     device, 
            //     queue, 
            //     format, 
            //     model_bind_group_layout, 
            //     text_bind_group_layout, 
            //     screen_width, 
            //     screen_height, 
            // ).unwrap()
        }
    }

    fn load_scene_from_hdf5(
        filepath: &str, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat, 
        model_bind_group_layout: &wgpu::BindGroupLayout, 
        text_bind_group_layout: &wgpu::BindGroupLayout,
        screen_width: u32,
        screen_height: u32,
    ) -> hdf5::Result<Scene> {
        let file = hdf5::File::open(filepath).unwrap();
        let vehicles = file.group("Vehicles").unwrap();
        let vehicles_vec = vehicles.groups().unwrap();
        let mut entity_vec = vec![];
        for vehicle in vehicles_vec.iter() {
            let name_full = vehicle.name();
            let name = name_full["/Vehicles/".len()..].to_string();
            let data = vehicle.dataset("states").unwrap();
            let entity = Entity::load_from_hdf5(name, data, device, model_bind_group_layout).unwrap();
            entity_vec.push(entity);
        }

        let num_entities = entity_vec.len();
        println!("LOADED {} ENTITIES INTO SCENE", num_entities);

        // set timesteps
        let timesteps = Scene::find_timesteps(&entity_vec);
        let data_counter = timesteps.map(|_| 0 as usize);


        Ok(Scene::new(
            entity_vec,
            timesteps,
            data_counter,
            device,
            queue,
            format,
            text_bind_group_layout,
            screen_width, 
            screen_height,
        ))
    }

    fn find_timesteps(entity_vec: &Vec<Entity>) -> Option<usize>{
        for entity in entity_vec.iter() {
            for behavior in entity.behaviors.iter() {
                if !behavior.is_constant_behavior {
                    println!("found non-constant behavior");
                    return Some(behavior.data.len())
                }
            }
        }

        println!("could only find constant behavior");
        None
    }

    fn load_scene_from_json(
        filepath: &str, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat, 
        model_bind_group_layout: &wgpu::BindGroupLayout, 
        text_bind_group_layout: &wgpu::BindGroupLayout,
        screen_width: u32,
        screen_height: u32,
    ) -> Scene {
        let json_unparsed = std::fs::read_to_string(filepath).unwrap();
        Scene::load_scene_from_json_str(
            json_unparsed, 
            device, 
            queue, 
            format, 
            model_bind_group_layout, 
            text_bind_group_layout, 
            screen_width, 
            screen_height,
        )
    }

    fn load_scene_from_json_str(
        json_unparsed: String, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat, 
        model_bind_group_layout: &wgpu::BindGroupLayout, 
        text_bind_group_layout: &wgpu::BindGroupLayout,
        screen_width: u32,
        screen_height: u32,
    ) -> Scene {
        let json: serde_json::Value = serde_json::from_str(&json_unparsed).unwrap();
        let timesteps = json["timesteps"].as_u64();
        let timesteps = timesteps.map(|e| e as usize);
        let data_counter = timesteps.map(|_| 0 as usize);

        let entity_temp: Vec<_> = json["entities"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        let mut entity_vec = vec![];
        for i in entity_temp.iter() {
            entity_vec.push(Entity::load_from_json(*i, device, model_bind_group_layout));
        }

        Scene::new(
            entity_vec,
            timesteps,
            data_counter,
            device,
            queue,
            format,
            text_bind_group_layout,
            screen_width, 
            screen_height,
        )
    }

    pub fn load_scene_from_network(
        addr: &str, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue,
        format: &wgpu::TextureFormat, 
        model_bind_group_layout: &wgpu::BindGroupLayout, 
        text_bind_group_layout: &wgpu::BindGroupLayout, 
        screen_width: u32,
        screen_height: u32,
    ) -> Result<Scene, Box<dyn std::error::Error>> {
        // Open port
        let listener = std::net::TcpListener::bind(addr).unwrap();
        let mut num_attempt = 0usize;
        
        // Attempt to recieve initialization packet and parse when successful.
        let initialization_packet = loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    // debug!("{}", com::from_network(&stream));
                    break com::from_network(&stream)
                },
                _ => {
                    num_attempt += 1;
                    debug!("No packet recieved. Trying attempt {}...", num_attempt);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                },
            }
        };
        info!("Received initialization file");
        let initialization_packet = String::from_utf8(initialization_packet).unwrap();
        // debug!("Initialization file: {}", initialization_packet);

        // Receive and save model files
        for stream in listener.incoming() {

            let mut local_stream = stream.unwrap();
            match com::from_network_with_protocol(&mut local_stream) {
                Ok(_) => {},
                Err("END") => {
                    debug!("Finished recieving files!");
                }
                _ => {break}
            }
        }

        info!("All files recieved.");

        //
        
        // Load Scene from initialization packet

        Ok(
            Scene::load_scene_from_json_str(
                initialization_packet, 
                device, 
                queue,
                format,
                model_bind_group_layout, 
                text_bind_group_layout,
                screen_width, 
                screen_height,
            )
        )
        
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        ortho_matrix_bind_group: &'a wgpu::BindGroup,
        model_render_pipeline: &'a wgpu::RenderPipeline,
        text_render_pipeline: &'a wgpu::RenderPipeline,
        terrain_render_pipeline: &'a wgpu::RenderPipeline,
        queue: &wgpu::Queue,
    ){
        render_pass.set_pipeline(terrain_render_pipeline);
        render_pass.draw_terrain(&self.terrain, camera_bind_group);

        render_pass.set_pipeline(model_render_pipeline);
        for entity in self.entities.iter() {
            entity.draw(render_pass, camera_bind_group, queue);
        }

        // println!("preparing to draw text");
        render_pass.set_pipeline(text_render_pipeline);
        for text_box in self.text_boxes.iter() {
            // println!("drawing text");
            text_box.draw(ortho_matrix_bind_group, render_pass);
        }
    }

    fn init_capture_buffers(device: &wgpu::Device, num_buffers: usize, size: wgpu::BufferAddress) -> Vec<wgpu::Buffer> {
        let buffers: Vec<wgpu::Buffer> = (0..num_buffers)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Frame readback buffer"),
                    size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            })
            .collect();
        
        buffers
    }

    pub fn read_and_write_capture_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, offscreen_texture: &wgpu::Texture, width: u32, height: u32){
            let index = self.frame_counter % NUM_CAPTURE_BUFFERS;
            // read buf n
            if self.frame_counter > NUM_CAPTURE_BUFFERS {
                let mut saved_frames = Scene::read_capture_buf(
                    device, 
                    &self.capture_buffers, 
                    width, 
                    height,
                    index,
                ).expect("Error in State::update(); failed to capture screen data in buffer");
                self.screen_recordings.append(&mut saved_frames);
            }

            // write to buf n
            Scene::write_screen_to_capture_buf(
                device,
                queue,
                offscreen_texture,
                &mut self.capture_buffers,
                width, 
                height,
                index,
            );

            self.increment_frame_counter();
            
            if self.data_counter > self.timesteps {
                println!("sim finished; time to save");
                self.read_remaining_buffers(device, width, height);
                println!("{} total frames recorded", self.screen_recordings.len());
                Scene::save_screen_data_to_file(&self.screen_recordings);
                // send window close event instead of panicking
                panic!();
            }

    }
    
    fn write_screen_to_capture_buf(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, capture_buffers: &mut Vec<wgpu::Buffer>, width: u32, height: u32, index: usize){
        let padded_bytes_per_row = ((width * BYTES_PER_PIXEL + 255) / 256) * 256;

        let capture_buf = &capture_buffers[index];

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("CopyTextureToBuffer Encoder"),
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &capture_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1
            },
        );

        // println!("wrote to buffer[{}]", index);
        // println!("oldest buffer is now {}", (index+1) % NUM_CAPTURE_BUFFERS);

        queue.submit(Some(encoder.finish()));
    }

    fn read_capture_buf(device: &wgpu::Device, capture_buffers: &Vec<wgpu::Buffer>, width: u32, height: u32, index: usize) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let padded_bytes_per_row = ((width * BYTES_PER_PIXEL + 255) / 256) * 256;
        let mut output = Vec::new();

        let buffer = &capture_buffers[index];
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        device.poll(wgpu::MaintainBase::Wait)?;
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * BYTES_PER_PIXEL) as usize);

        for chunk in data.chunks(padded_bytes_per_row as usize) {
            pixels.extend_from_slice(&chunk[..(width * BYTES_PER_PIXEL) as usize]);
        }

        drop(data);
        buffer.unmap();
        output.push(pixels);

        // println!("read from buffer[{}]", index);

        Ok(output)
    }

    fn read_remaining_buffers(&mut self, device: &wgpu::Device, width: u32, height: u32){
        for i in 0..NUM_CAPTURE_BUFFERS {
            let index = (self.frame_counter + i) % NUM_CAPTURE_BUFFERS;
            let mut saved_frame = Scene::read_capture_buf(device, &self.capture_buffers, width, height, index).expect("problem with reading final few buffers");
            self.screen_recordings.append(&mut saved_frame);
        }
    }

    fn save_screen_data_to_file(screen_data: &Vec<Vec<u8>>){

        let mut ffmpeg_process = Command::new("ffmpeg")
            .args(&[
                "-f", "rawvideo", //    input is raw video pixels
                "-pix_fmt", "rgba", //  RGBA format
                "-s", "1600x1200", //   dimensions
                "-r", "60", //          fps
                "-i", "pipe:0", //      read input from stdin
                "-c:v", "libx264", //   specify video codex
                "-pix_fmt", "yuv420p",
                "-preset", "fast", // fast encoding preset
                "-movflags", "faststart", // optimize for streaming
                "-y", // overwrite output file
                "output.mp4",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("ffmpeg process failed to execute");

        let mut stdin = ffmpeg_process.stdin.take().expect("failed to extract stdin from ffmpeg process");

        for frame in screen_data {
            stdin.write_all(frame).expect("failed to write input to ffmpeg process");
        }
        drop(stdin);

        let status = ffmpeg_process.wait().unwrap();

        if status.success() {
            println!("video successfully converted!");
        } else {
            eprintln!("ffmpeg failed with status: {:?}", status)
        }
    }
}