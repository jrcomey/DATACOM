// SCENES AND ENTITIES

use crate::dc;
use nalgebra as na;
use tobj::Model;
use std::{sync::atomic::{AtomicU64, Ordering}, vec};

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
    wireframe: WireframeObject,
}

impl ModelComponent {

    pub fn new(wireframe: WireframeObject) -> ModelComponent {
        ModelComponent {
            wireframe: wireframe
        }
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
    position: na::Point3<f32>,
    rotation: na::UnitQuaternion<f32>,
    scale: na::Vector3<f32>,
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
}

impl dc::Draw2 for Entity {
    
    fn draw(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame) {
        
    }
}

// Define the scene structure
pub struct Scene {
    entities: Vec<Entity>,
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
}

pub trait DrawInScene {
    fn draw_at_position(&self, gui: &dc::GuiContainer, context: &dc::RenderContext, target: &mut glium::Frame, model: na::Matrix4<f32>);
}