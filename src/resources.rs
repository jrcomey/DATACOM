use std::io::{BufReader, Cursor};

use wgpu::util::DeviceExt;

use crate::model;

pub async fn load_mesh(
    file_name: &str,
    device: &wgpu::Device,
    color: 
) -> anyhow::Result<model::Mesh> {
    let obj_cursor = Cursor::new(file_name);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, _) = tobj::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| {
            let mat_text = p.to_string_lossy().to_string();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )?;

    let model = &models[0];
    let mesh = &model.mesh;

    let mut vertices = Vec::new();
    let positions = &mesh.positions;
    for i in 0..positions.len() / 3 {
        let position = [
            mesh.positions[i*3],
            mesh.positions[i*3 + 1],
            mesh.positions[i*3 + 2],
        ];

        // TODO: allow custom colors
        let color = [1.0, 0.0, 0.0];

        vertices.push(model::ModelVertex {
            position,
            color,
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

    Ok(model::Mesh {
        name: model.name.clone(),
        vertex_buffer,
        index_buffer,
        num_elements: mesh.indices.len() as u32,
    })
}
