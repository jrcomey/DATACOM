use crate::dc;

pub struct Wireframe {
    positions:  Vec<dc::Vertex>,
    normals:    Vec<dc::Normal>,
    indices:    Vec<u32>,
    color:      na::base::Vector4<f32>
}

impl Wireframe {
    pub fn load_wireframe_from_obj(filepath: &str, colorvec: na::base::Vector4<f32>) -> Wireframe {
        let file = tobj::load_obj(filepath, &tobj::GPU_LOAD_OPTIONS);
        assert!(file.is_ok());
        
        let (models, _) = file.unwrap();
        let mesh = &models[0].mesh;
    
        Wireframe{
            positions: Wireframe::convert_to_vertex_struct(&mesh.positions),
            normals: Wireframe::convert_to_normal_struct(&mesh.normals),
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

    pub fn new(positions: Vec<dc::Vertex>, normals: Vec<dc::Normal>, indices: Vec<u32>, color: na::base::Vector4<f32>) -> Wireframe {
        Wireframe { positions: positions, normals: normals, indices: indices, color: color }
    }

}

impl dc::Draw for Wireframe {

    fn draw(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
        // info!("Drawing Wireframe");
        use glium::Surface;

        let uniforms = glium::uniform! {
            model: dc::uniformifyMat4(mvp.model),
            view: dc::uniformifyMat4(mvp.view),
            perspective: dc::uniformifyMat4(mvp.perspective),
            color_obj: dc::uniformifyVec4(self.color),
            vp: dc::uniformifyMat4(mvp.vp),
            bounds: dc::uniformifyVec4(mvp.bounds),
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
            scissor: std::option::Option::Some(mvp.pixel_box),
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

    fn draw_absolute(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
        error!("Not implemented!");
    }

}