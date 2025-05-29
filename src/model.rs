use wgpu::util::DeviceExt;
use cgmath::EuclideanSpace;

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Mesh {
    #[allow(unused)]
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

pub fn load_mesh(
    file_name: &str,
    device: &wgpu::Device,
    color: cgmath::Vector3<f32>,
) -> anyhow::Result<Mesh> {
    let (models, _) = tobj::load_obj(
        file_name,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )?;


    let model = &models[0];
    let mesh = &model.mesh;

    let color_arr = [color[0], color[1], color[2]];
    let mut vertices = Vec::new();
    let positions = &mesh.positions;
    for i in 0..positions.len() / 3 {
        let position = [
            mesh.positions[i*3],
            mesh.positions[i*3 + 1],
            mesh.positions[i*3 + 2],
        ];

        vertices.push(ModelVertex {
            position: position,
            color: color_arr,
        });


    }

    let vertex_data = bytemuck::cast_slice(&vertices);
    let index_data = bytemuck::cast_slice(&mesh.indices);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: vertex_data,
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: index_data,
        usage: wgpu::BufferUsages::INDEX,
    });

    Ok(Mesh {
        name: model.name.clone(),
        vertex_buffer,
        index_buffer,
        num_elements: mesh.indices.len() as u32,
    })
}

#[allow(dead_code)]
pub struct Model {
    pub name: String,
    pub obj: Mesh,
    pub position: cgmath::Point3<f32>,
    pub orientation: cgmath::Quaternion<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub color: cgmath::Vector3<f32>,
}

impl Model {
    pub fn new(
        name: &str, 
        filepath: &str, 
        device: &wgpu::Device,
        position: cgmath::Point3<f32>, 
        orientation: cgmath::Quaternion<f32>, 
        rotation: cgmath::Quaternion<f32>, 
        color: cgmath::Vector3<f32>,
    ) -> Model {
        let mesh = load_mesh(filepath, device, color)
        .expect("Failed to load mesh in Model::new()");
        Model {
            name: name.to_string(),
            obj: mesh,
            position: position,
            orientation: orientation,
            rotation: rotation,
            color: color,
        }
    }

    pub fn load_from_json(json: &serde_json::Value, device: &wgpu::Device) -> Model {
        let name = json["Name"].as_str().unwrap();
        let filepath = json["ObjectFilePath"].as_str().unwrap();

        let mut position_vec = cgmath::Point3::<f32>::new(0.0, 0.0, 0.0);
        let position_temp: Vec<_> = json["Position"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        for (i, position) in position_temp.iter().enumerate() {
            position_vec[i] = position.as_f64().unwrap() as f32;
        }

        let orientation_temp: Vec<_> = json["Orientation"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        let mut orientation_vec = cgmath::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, orientation_comp) in orientation_temp.iter().enumerate() {
            orientation_vec[i] = orientation_comp.as_f64().unwrap() as f32;
        }

        let rotation_temp: Vec<_> = json["Rotation"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        let mut rotation_vec = cgmath::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, rotation_comp) in rotation_temp.iter().enumerate() {
            rotation_vec[i] = rotation_comp.as_f64().unwrap() as f32;
        }

        let color_temp: Vec<_> = json["Color"]
            .as_array()
            .unwrap()
            .into_iter()
            .collect();
        let mut color_vec = cgmath::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for (i, color_comp) in color_temp.iter().enumerate() {
            color_vec[i] = color_comp.as_f64().unwrap() as f32;
        }

        // debug!("NAME: {}", name);
        // debug!("POSITION: {}", position_vec);
        // debug!("ORIENTATION: {}", orientation_vec);
        // debug!("ROTATION: {}", rotation_vec);
        // debug!("COLOR: {}", color_vec);

        Model::new(
            name,
            filepath,
            device,
            position_vec,
            cgmath::Quaternion::from_sv(1.0, orientation_vec),
            cgmath::Quaternion::from_sv(1.0, rotation_vec),
            color_vec,
        )
    }

    pub fn to_matrix(&self) -> cgmath::Matrix4<f32> {
        let translation = cgmath::Matrix4::from_translation(self.position.to_vec());
        let rotation = cgmath::Matrix4::from(self.rotation);
        translation * rotation
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        camera_bind_group: &'b wgpu::BindGroup,
        model_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, model_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, 0..1);
    }
}