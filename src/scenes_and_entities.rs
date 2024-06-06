// SCENES AND ENTITIES

use crate::{dc::{self, green_vec}, wf::Wireframe};
use glium::debug;
use nalgebra as na;
use num_traits::ToPrimitive;
// use rand::Error;
use tobj::Model;
use std::{ops::DerefMut, sync::atomic::{AtomicU64, Ordering}, vec};
use serde_json::{Value};

// Define the wireframe or model representation
pub struct WireframeObject {
    // Define the structure for your wireframe representation
    // This could include vertex data, indices, color, etc.
    // ...

    positions:  Vec<dc::Vertex>,
    normals:    Vec<dc::Normal>,
    indices:    Vec<u32>,
    color:      na::base::Vector4<f32>
}

impl WireframeObject {
    pub fn load_wireframe_from_obj(filepath: &str, colorvec: na::base::Vector4<f32>) -> WireframeObject {
        let file = tobj::load_obj(filepath, &tobj::GPU_LOAD_OPTIONS);
        assert!(file.is_ok());
        
        let (models, _) = file.unwrap();
        let mesh = &models[0].mesh;
    
        WireframeObject{
            positions: WireframeObject::convert_to_vertex_struct(&mesh.positions),
            normals: WireframeObject::convert_to_normal_struct(&mesh.normals),
            indices: mesh.indices.clone(),
            color: colorvec
        }
    }

    pub fn convert_to_vertex_struct (target: &Vec<f32>) -> Vec<dc::Vertex>{
        // Have to pass a reference (&) to the target. 
        // Datatype does not support copying, and Rust creates a copy of the orignal function arguments. 
        // Original copy not modified, so fine
        let mut vertex_array: Vec<dc::Vertex> = std::vec::Vec::new();
        for i in 0..target.len()/3 {
            vertex_array.push(dc::Vertex::new(target[3*i+0] as f64, target[3*i+1] as f64, target[3*i+2] as f64));
        };
        return vertex_array;
    }
    
    pub fn convert_to_normal_struct (target: &Vec<f32>) -> Vec<dc::Normal>{
        // Have to pass a reference (&) to the target. 
        // Datatype does not support copying, and Rust creates a copy of the orignal function arguments. 
        // Original copy not modified, so fine
        let mut normal_array: Vec<dc::Normal> = std::vec::Vec::new();
        for i in 0..target.len()/3 {
            normal_array.push(dc::Normal::new(target[3*i+0] as f64, target[3*i+1] as f64, target[3*i+2] as f64));
        }
        return normal_array;
    }

    pub fn new(positions: Vec<dc::Vertex>, normals: Vec<dc::Normal>, indices: Vec<u32>, color: na::base::Vector4<f32>) -> WireframeObject {
        WireframeObject { positions: positions, normals: normals, indices: indices, color: color }
    }

    pub fn change_color(&mut self, new_color: na::Vector4<f32>) {
        self.color = new_color;
    }

    pub fn get_color(&mut self) -> na::Vector4<f32> {
        self.color
    }

}

impl DrawInScene for WireframeObject {

    fn draw_at_position(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame, model: na::Matrix4<f32>) {
        // info!("Drawing Wireframe");
        use glium::Surface;

        let uniforms = glium::uniform! {
            model: dc::uniformifyMat4(model),
            view: dc::uniformifyMat4(context.view),
            perspective: dc::uniformifyMat4(context.perspective),
            color_obj: dc::uniformifyVec4(self.color),
            vp: dc::uniformifyMat4(context.viewport_shift),
        };

        let positions = glium::VertexBuffer::new(&gui.display, &self.positions).unwrap();
        let normals = glium::VertexBuffer::new(&gui.display, &self.normals).unwrap();
        let indices = glium::IndexBuffer::new(&gui.display, glium::index::PrimitiveType::TrianglesList, &self.indices).unwrap();

        let poly_off = glium::draw_parameters::PolygonOffset{
            factor: 1.0,
            units: 1.0,
            point: true,
            line: true,
            fill: true
        };

        let params = glium::DrawParameters {
            polygon_mode: glium::draw_parameters::PolygonMode::Line,
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            line_width: std::option::Option::Some(1E-5),
            scissor: std::option::Option::Some(context.pixel_bounds),
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            polygon_offset: poly_off,
            ..Default::default()
        };

        target.draw(
            (
                &positions, 
                &normals
            ), 
            &indices, 
            &gui.program, 
            &uniforms, 
            &params)
                .unwrap();
    }
}

// Define the component for a model
pub struct ModelComponent {
    wireframe: WireframeObject,                         // Wireframe model of the object to be displayed
    local_position: na::Point3<f32>,                    // Local **unscaled** position relative to parent body
    local_orientation: na::UnitQuaternion<f32>,         // Primary rotation of model relative to parent body. Intended to be static but can be modified.
    local_rotation: na::UnitQuaternion<f32>,            // Secondary rotation of model relative to parent body. Intended to be a dynamic property.
}

impl ModelComponent {

    pub fn new(wireframe: WireframeObject) -> ModelComponent {
        ModelComponent {
            wireframe: wireframe,
            local_position: na::Point3::origin(),
            local_orientation: na::UnitQuaternion::identity(),
            local_rotation: na::UnitQuaternion::identity()
        }
    }

    pub fn load_from_json_file(filepath: &str) -> ModelComponent {
        let json_unparsed = std::fs::read_to_string(filepath).unwrap();
        ModelComponent::load_from_json_str(&json_unparsed)
    }

    pub fn load_from_json_str(json_string: &str) -> ModelComponent {
        let json_parsed: Value = serde_json::from_str(json_string).unwrap();
        ModelComponent::load_from_json(&json_parsed)
    }

    pub fn load_from_json(json_parsed: &serde_json::Value) -> ModelComponent{
        let name = json_parsed["Name"].as_str().unwrap();
        let filepath = json_parsed["ObjectFilePath"].as_str().unwrap();
        
        
        let mut position_vec = na::Point3::<f32>::new(0.0,0.0,0.0);
        let position_temp: Vec<_> = json_parsed["Position"].as_array().unwrap().into_iter().collect();
        for (i, position) in position_temp.iter().enumerate() {
            position_vec[i] = position.as_f64().unwrap() as f32;
        }

        let orientation_temp: Vec<_> = json_parsed["Orientation"].as_array().unwrap().into_iter().collect();
        let mut orientation_vec = na::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, orientation_comp) in orientation_temp.iter().enumerate() {
            orientation_vec[i] = orientation_comp.as_f64().unwrap() as f32;
        }

        let rotation_temp: Vec<_> = json_parsed["Rotation"].as_array().unwrap().into_iter().collect();
        let mut rotation_vec = na::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, rotation_comp) in rotation_temp.iter().enumerate() {
            rotation_vec[i] = rotation_comp.as_f64().unwrap() as f32;
        }

        let color_temp: Vec<_> = json_parsed["Color"].as_array().unwrap().into_iter().collect();
        let mut color_vec = na::Vector4::<f32>::new(0.0, 0.0, 0.0, 0.0);
        for (i, color_comp) in color_temp.iter().enumerate() {
            color_vec[i] = color_comp.as_f64().unwrap() as f32;
        }


        // debug!("NAME: {}", name);
        // debug!("POSITION: {}", position_vec);
        // debug!("ORIENTATION: {}", orientation_vec);
        // debug!("ROTATION: {}", rotation_vec);
        // debug!("COLOR: {}", color_vec);




        ModelComponent {
            wireframe: WireframeObject::load_wireframe_from_obj(&filepath, color_vec),
            local_position: position_vec,
            local_orientation: na::UnitQuaternion::new(orientation_vec),
            local_rotation: na::UnitQuaternion::new(rotation_vec)
        }
    }

    pub fn update_local_position(&mut self, new_local_position: na::Point3<f32>) {
        self.local_position = new_local_position;
    }

    pub fn update_local_rotation(&mut self, new_local_rotation: na::UnitQuaternion<f32>) {
        self.local_rotation = new_local_rotation;
    }

    pub fn rotate_by(&mut self, rotation_factor: na::UnitQuaternion<f32>){
        self.local_rotation = self.local_rotation*rotation_factor;
    }

    pub fn change_color(&mut self, new_color: na::Vector4<f32>) {
        self.wireframe.change_color(new_color);
    }

    pub fn get_color(&mut self) -> na::Vector4<f32> {
        self.wireframe.get_color()
    }
}

impl DrawInScene for ModelComponent {
    fn draw_at_position(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame, model: na::Matrix4<f32>) {
        let local_model = na::Isometry3::from_parts(
            na::Translation3::from(self.local_position), 
            self.local_rotation
        ).to_homogeneous();

        self.wireframe.draw_at_position(
            gui, 
            context, 
            target, 
            model*local_model);
    }
}

// Define the component for a behavior
pub struct BehaviorComponent {
    // Define the behavior-specific data and logic
    // This could include methods for movement, rotation, etc.
    // Intended to be used for constant animations: e.g. spinning rotors that
    // don't need to wait for external command but should perform anyway
    pub behavior: Command
}

impl BehaviorComponent {

    pub fn new(behavior_vector: Command) -> BehaviorComponent {
        BehaviorComponent {
            behavior: behavior_vector
        }
    }

    pub fn get_behavior(&self) -> Command{
        self.behavior.clone()
    }

    pub fn change_command_data(&mut self, new_data: Vec<f64>) {
        self.behavior.change_data(new_data);
    }

    pub fn load_from_json(json_parsed: &serde_json::Value) -> BehaviorComponent {

        let command = Command::from_json(json_parsed);
        BehaviorComponent { behavior: command }
        
    }

}

static ENTITY_COUNTER: AtomicU64 = AtomicU64::new(0);

// Define the entity structure
pub struct Entity {
    id: u64,  // Unique identifier for the entity
    position: na::Point3<f64>,
    rotation: na::UnitQuaternion<f64>,
    scale: na::Vector3<f64>,
    models: Vec<ModelComponent>,
    behaviors: Vec<BehaviorComponent>,
    // Other entity-specific data...
}

impl Entity {
    // Additional methods for entity management...

    pub fn new() -> Entity {
        Entity {
            id: ENTITY_COUNTER.fetch_add(1, Ordering::Relaxed),
            position: na::Point3::origin(),
            rotation: na::UnitQuaternion::identity(),
            scale: na::Vector3::new(1.0, 1.0, 1.0),
            models: Vec::new(),
            behaviors: Vec::new(),
            // Other entity-specific data...
        }
    }

    pub fn load_from_json_file(filepath: &str) -> Entity{
        let json_unparsed = std::fs::read_to_string(filepath).unwrap();
        Entity::load_from_json_str(&json_unparsed)
    }

    pub fn load_from_json_str(json_string: &str) -> Entity {
        let json_parsed: Value = serde_json::from_str(json_string).unwrap();
        Entity::load_from_json(&json_parsed)
    }

    pub fn load_from_json(json_parsed: &serde_json::Value) -> Entity{
        let name = json_parsed["Name"].as_str().unwrap();
        let mut position_vec = na::Point3::<f64>::new(0.0,0.0,0.0);
        let position_temp: Vec<_> = json_parsed["Position"].as_array().unwrap().into_iter().collect();
        for (i, position) in position_temp.iter().enumerate() {
            position_vec[i] = position.as_f64().unwrap();
        }

        let rotation_temp: Vec<_> = json_parsed["Rotation"].as_array().unwrap().into_iter().collect();
        let mut rotation_vec = na::Vector3::<f64>::new(0.0, 0.0, 0.0);
        for (i, rotation_comp) in rotation_temp.iter().enumerate() {
            rotation_vec[i] = rotation_comp.as_f64().unwrap();
        }

        let scale_temp: Vec<_> = json_parsed["Scale"].as_array().unwrap().into_iter().collect();
        let mut scale_vec = na::Vector3::<f64>::new(0.0, 0.0, 0.0);
        for (i, scale_comp) in scale_temp.iter().enumerate() {
            scale_vec[i] = scale_comp.as_f64().unwrap();
        }

        // let scale_vec: Vec<_> = match json_parsed["Scale"].as_array() {
        //     Some(array) => {
        //         let scale_temp: Vec<_> = array.into_iter().collect();
        //         let mut scale_vec = na::Vector3::<f64>::new(0.0, 0.0, 0.0);
        //         for (i, scale_comp) in scale_temp.iter().enumerate() {
        //             scale_vec[i] = scale_comp.as_f64().unwrap();
        //         }
        //         scale_vec
        //     },
        //     None => vec![]
        // };

        // let model_temp: Vec<_> = json_parsed["Models"].as_array().unwrap().into_iter().collect();        
        // let mut model_vec = vec![];
        // for i in model_temp.iter() {
        //     model_vec.push(ModelComponent::load_from_json(*i));
        // }

        let model_vec: Vec<_> = match json_parsed["Models"].as_array() {
            Some(array) => {
                let model_temp: Vec<_> = array.into_iter().collect();        
                let mut model_vec = vec![];
                for i in model_temp.iter() {
                    model_vec.push(ModelComponent::load_from_json(*i));
                }
                model_vec
            },
            None => vec![]
        };

        // let behavior_temp: Vec<_> = json_parsed["Behaviors"].as_array().unwrap().into_iter().collect();
        // let mut behavior_vec = vec![];
        // for i in behavior_temp.iter() {
        //     behavior_vec.push(BehaviorComponent::load_from_json(*i));
        // }

        let behavior_vec: Vec<_> = match json_parsed["Behaviors"].as_array() {
            Some(array) => {
                let behavior_temp: Vec<_> = array.into_iter().collect();
                let mut behavior_vec = vec![];
                for i in behavior_temp.iter() {
                    behavior_vec.push(BehaviorComponent::load_from_json(*i));
                }
                behavior_vec
            },
            None => vec![]
        };
        // debug!("{}", model_temp[0]["Name"]);

        // debug!("NAME: {}", name);
        // debug!("POSITION: {}", position_vec);
        // debug!("ROTATION: {}", rotation_vec);
        // debug!("SCALE: {}", scale_vec);
        

        Entity {
            id: ENTITY_COUNTER.fetch_add(1, Ordering::Relaxed),
            position: position_vec,
            rotation: na::UnitQuaternion::new(rotation_vec),
            scale: scale_vec,
            models: model_vec,
            behaviors: behavior_vec,
            // Other entity-specific data...
        }
    }

    pub fn add_model(&mut self, model: ModelComponent) {
        self.models.push(model);
    }

    pub fn add_behavior(&mut self, behavior: BehaviorComponent) {
        self.behaviors.push(behavior);
    }

    pub fn change_position(&mut self, new_position: na::Point3<f64>) {
        self.position = new_position;
    }

    pub fn change_scale(&mut self, new_scale: na::Vector3<f64>) {
        self.scale = new_scale;
    }

    pub fn get_model(&mut self, model_component_id: u64) -> &mut ModelComponent {
        &mut self.models[model_component_id as usize]
    }

    pub fn get_behavior(&mut self, index: usize) -> Command {
        self.behaviors[index].get_behavior()
    }
    
    pub fn command(&mut self, cmd: Command) {
        
        match cmd.cmd_type {

            // Translate entity by vector
            CommandType::EntityTranslate => {
                let new_position = na::Vector3::<f64>::new(cmd.data[0], cmd.data[1], cmd.data[2]);
                self.change_position(self.position+new_position);
            },

            // Change position to input
            CommandType::EntityChangePosition => {
                let new_position = na::Point3::<f64>::new(cmd.data[0], cmd.data[1], cmd.data[2]);
                self.change_position(new_position);
            },

            // Change color to input
            CommandType::ComponentChangeColor => {
                let model_id = cmd.data[0] as u64;
                let new_color = na::Vector4::<f32>::new(
                    cmd.data[1] as f32,
                    cmd.data[2] as f32,
                    cmd.data[3] as f32,
                    cmd.data[4] as f32,
                );
                self.get_model(model_id).change_color(new_color);
            },

            // Rotate item at constant speed
            CommandType::ComponentRotateConstantSpeed => {
                let model_id = cmd.data[0] as u64;
                let rotation_factor = cmd.data[1];
                let new_quaternion_vector = na::Vector3::<f32>::new(
                    (rotation_factor*cmd.data[2]) as f32,
                    (rotation_factor*cmd.data[4]) as f32,
                    (rotation_factor*cmd.data[3]) as f32,
                );
                let new_quaternion = na::UnitQuaternion::<f32>::new(
                    new_quaternion_vector
                );

                self.get_model(model_id).rotate_by(
                    new_quaternion
                );
            }

            // Modify Existing Behavior
            CommandType::ModifyBehavior => {
                let behavior_id = cmd.data[0] as usize;
                let new_data = cmd.data.into_iter().collect();
                self.get_behavior(behavior_id).change_data(new_data);
            }



            _ => return,
        }
    }

    pub fn clear_behaviors(&mut self) {
        self.behaviors = vec![];
    }

    pub fn run_behaviors(&mut self) {

        // Gather behaviors into vector

        let mut cmds = vec![];
        for i in &mut self.behaviors {
            cmds.push(i.get_behavior());
        }
        //Run them all in an iterator
        
        for i in cmds.iter(){
            self.command(i.clone());
        }
    }

    pub fn get_position(&mut self) -> na::Point3<f64> {
        self.position
    }
}

impl dc::Draw2 for Entity {
    
    fn draw(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame) {
        let translate = na::Translation3::from(self.position);
        let parent_model_mat = na::Isometry3::from_parts(na::convert(translate), na::convert(self.rotation));
        let scale_vec = na::Vector3::new(self.scale.x as f32, self.scale.y as f32, self.scale.z as f32);
        for model in &self.models {
            model.draw_at_position(gui, context, target, parent_model_mat.to_homogeneous().prepend_nonuniform_scaling(&scale_vec));
        }
    }
}

// Types of command
#[derive(Copy, Clone)]
pub enum CommandType {
    EntityRotate,
    EntityTranslate,
    EntityChangePosition,
    ComponentRotate,
    ComponentTranslate,
    ComponentChangeColor,
    ComponentRotateConstantSpeed,
    ModifyBehavior,
    Null,
}

impl CommandType {
    pub fn match_from_string(input_string: &str) -> CommandType {
        match input_string {
            "EntityRotate" => CommandType::EntityRotate,
            "EntityTranslate" => CommandType::EntityTranslate,
            "EntityChangePosition" => CommandType::EntityChangePosition,
            "ComponentRotate" => CommandType::ComponentRotate,
            "ComponentTranslate" => CommandType::ComponentTranslate,
            "ComponentChangeColor" => CommandType::ComponentChangeColor,
            "ComponentRotateConstantSpeed" => CommandType::ComponentRotateConstantSpeed,
            "ModifyBehavior" => CommandType::ModifyBehavior,
            _ => CommandType::Null,
        }
    }
}
#[derive(Clone)]
pub struct Command {
    pub cmd_type: CommandType,
    pub data: Vec<f64>
}

impl Command {
    pub fn new(cmd_type: CommandType, data: Vec<f64>) -> Command {
        Command {
            cmd_type: cmd_type,
            data: data
        }
    }

    pub fn change_data(&mut self, new_data: Vec<f64>) {
        self.data = new_data;
    }

    pub fn get_data(&self) -> &Vec<f64> {
        &self.data
    }

    pub fn from_json(json_parsed: &serde_json::Value) -> Command {

        let data_temp: Vec<_> = json_parsed["data"].as_array().unwrap().into_iter().collect();
        let mut data: Vec<f64> = vec![];
        for (i, data_point) in data_temp.iter().enumerate() {
            data.push(data_point.as_f64().unwrap());
        }

        // let data: Vec<_> = match json_parsed["data"].as_array() {
        //     Some(data_temp) => {
        //         let data_temp: Vec<_> = json_parsed["data"].as_array().unwrap().into_iter().collect();
        //         let mut data: Vec<f64> = vec![];
        //         for (i, data_point) in data_temp.iter().enumerate() {
        //             data.push(data_point.as_f64().unwrap());
        //         }
        //         data
        //     },
        //     None => vec![],
        // };
    
        let command_type: CommandType = CommandType::match_from_string(json_parsed["commandType"].as_str().unwrap());

        Command::new(command_type, data)
    }
}

// Define the scene structure
pub struct Scene {
    pub entities: Vec<Entity>,
}

impl Scene {
    // Method to add an entity to the scene
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    // Other scene-related methods...
    pub fn new() -> Scene {
        Scene{
            entities: vec![]
        }
    }

    pub fn new_entities(entities: Vec<Entity>) -> Scene{
        Scene {
            entities: entities
        }
    }

    pub fn change_entity_position(&mut self, entity_id: u64, new_position: na::Point3<f64>) {
        self.entities[entity_id as usize].change_position(new_position);
    }

    pub fn change_entity_scale(&mut self, entity_id: u64, new_scale: na::Vector3<f64>) {
        self.entities[entity_id as usize].change_scale(new_scale);
    }

    pub fn update(&mut self) {

        // for entity in self.entities.iter().collect::<Vec<_>>() {
        //     entity.run_behaviors();
        // }

        // self.entities.iter().map(|x| x.run_behaviors()).next();

        // This is really dumb, but this is the only way to do it without cloning the data.
        for i in 0..self.entities.len(){
            self.get_entity(i).run_behaviors();
        }
    }

    pub fn cmd_msg_str(&mut self, json_unparsed: &str) {
        if json_unparsed.is_empty() {
            return
        }
        // let json_parsed: Value = serde_json::from_str(json_unparsed);
        // self.cmd_msg(&json_parsed);

        let json_parsed: Value = match serde_json::from_str(&json_unparsed) {
            serde_json::Result::Ok(val) => {val},
            serde_json::Result::Err(err) => {serde_json::Value::Null},
            // _ => {}
        };

        if json_parsed != serde_json::Value::Null {
            self.cmd_msg(&json_parsed);
        } else {
            error!("json failed to load!");
            error!("{}", json_unparsed);
        }
        
    }

    pub fn cmd_msg(&mut self, json_parsed: &serde_json::Value) {
        let target_entity_id = json_parsed["targetEntityID"].as_u64().unwrap() as usize;

        let cmd = Command::from_json(json_parsed);

        self.get_entity(target_entity_id).command(cmd);
    }

    
    pub fn load_from_json_file(filepath: &str) -> Scene {
        let json_unparsed = std::fs::read_to_string(filepath).unwrap();
        Scene::load_from_json_str(&json_unparsed)
    }

    pub fn load_from_json_str(json_unparsed: &str) -> Scene {
        let json_parsed: Value = serde_json::from_str(json_unparsed).unwrap();
        Scene::load_from_json(&json_parsed)
    }

    pub fn load_from_json(json_parsed: &serde_json::Value) -> Scene {
        let entity_temp: Vec<_> = json_parsed["entities"].as_array().unwrap().into_iter().collect();        
        let mut entity_vec = vec![];
        for i in entity_temp.iter() {
            entity_vec.push(Entity::load_from_json(*i));
        }

        Scene {
            entities: entity_vec
        }
    }
    pub fn get_entity(&mut self, entity_id: usize) -> &mut Entity {
        &mut self.entities[entity_id]
    }
}

impl dc::Draw2 for Scene {
    fn draw(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame) {
        for entity in &self.entities {
            entity.draw(gui, context, target);
        }
    }
}

pub trait DrawInScene {
    fn draw_at_position(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame, model_mat: na::Matrix4<f32>);
}