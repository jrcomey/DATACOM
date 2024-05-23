// SCENES AND ENTITIES

use crate::{dc::{self, green_vec}, wf::Wireframe};
use nalgebra as na;
use num_traits::ToPrimitive;
use rand::Error;
use tobj::Model;
use std::{ops::DerefMut, sync::atomic::{AtomicU64, Ordering}, vec};
use serde_json::{Result, Value};

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

        let name = json_parsed["Name"].as_str().unwrap();
        let filepath = json_parsed["ObjectFilePath"].as_str().unwrap();
        let position_temp: Vec<_> = json_parsed["Position"].as_array().unwrap().into_iter().collect();
        let orientation_temp: Vec<_> = json_parsed["Orientation"].as_array().unwrap().into_iter().collect();
        let rotation_temp: Vec<_> = json_parsed["Rotation"].as_array().unwrap().into_iter().collect();
        let color_temp: Vec<_> = json_parsed["Color"].as_array().unwrap().into_iter().collect();
        let mut position_vec = na::Point3::<f32>::new(0.0,0.0,0.0);
        for (i, position) in position_temp.iter().enumerate() {
            position_vec[i] = position.as_f64().unwrap() as f32;
        }

        let mut orientation_vec = na::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, orientation_comp) in orientation_temp.iter().enumerate() {
            orientation_vec[i] = orientation_comp.as_f64().unwrap() as f32;
        }

        let mut rotation_vec = na::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, rotation_comp) in rotation_temp.iter().enumerate() {
            rotation_vec[i] = rotation_comp.as_f64().unwrap() as f32;
        }

        let mut color_vec = na::Vector4::<f32>::new(0.0, 0.0, 0.0, 0.0);
        for (i, color_comp) in color_temp.iter().enumerate() {
            color_vec[i] = color_comp.as_f64().unwrap() as f32;
        }


        debug!("NAME: {}", name);
        debug!("POSITION: {}", position_vec);
        debug!("ORIENTATION: {}", orientation_vec);
        debug!("ROTATION: {}", rotation_vec);
        debug!("COLOR: {}", color_vec);




        ModelComponent {
            wireframe: WireframeObject::load_wireframe_from_obj(&filepath, color_vec),
            local_position: position_vec,
            local_orientation: na::UnitQuaternion::identity(),
            local_rotation: na::UnitQuaternion::identity()
        }
    }

    pub fn update_local_position(&mut self, new_local_position: na::Point3<f32>) {
        self.local_position = new_local_position;
    }

    pub fn update_local_rotation(&mut self, new_local_rotation: na::UnitQuaternion<f32>) {
        self.local_rotation = new_local_rotation;
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

    pub fn add_model(&mut self, model: ModelComponent) {
        self.models.push(model);
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
}

impl dc::Draw2 for Entity {
    
    fn draw(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame) {
        let translate = na::Translation3::from(self.position);
        let parent_model_mat = na::Isometry3::from_parts(na::convert(translate), na::convert(self.rotation));
        for model in &self.models {
            model.draw_at_position(gui, context, target, parent_model_mat.to_homogeneous());
        }
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

    pub fn change_entity_position(&mut self, entity_id: u64, new_position: na::Point3<f64>) {
        self.entities[entity_id as usize].change_position(new_position);
    }

    pub fn change_entity_scale(&mut self, entity_id: u64, new_scale: na::Vector3<f64>) {
        self.entities[entity_id as usize].change_scale(new_scale);
    }

    pub fn update(&mut self) {
        ;
    }

    pub fn get_entity(&mut self, entity_id: u64) -> &mut Entity {
        &mut self.entities[entity_id as usize]
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