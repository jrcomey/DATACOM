use glium::{debug, implement_vertex, Surface};
use glium::{texture::RawImage2d, Display, Texture2d, glutin::{self, surface::WindowSurface}};
use rusttype as rt;
use image;
use rt::{Font, Scale, point};
use std::collections::HashMap;
use std::sync::Arc;
use dc::Rgba;

use crate::dc::{self, uniformify_mat4, uniformify_vec4, Draw2};

/// Single character in the font. Has texture coordinates, size, bearing, and an advance.
pub struct Glyph {
    tex_coords: [f32; 4],           // tex_coords coordinates in texture atlas
    size: [f32; 2],         // Glyph width and height in pixels
    bearing: [f32; 2],      // Offset for positioning
    advance: f32,           // Horizontal advance after rendering
}

/// Function to load font atlas. Loads a given font. 
pub fn load_font_atlas(path: &str, font_size: f32) -> (image::RgbaImage, HashMap<char, Glyph>) {
    let font_data = std::fs::read(path).expect("Failed to read font file");
    let font = Font::try_from_vec(font_data).expect("Failed to load font");
    debug!("Font info: {:?}", font);

    let scale = Scale::uniform(font_size);
    let v_metrics = font.v_metrics(scale);

    let chars: Vec<char> = (' '..='~').collect(); // ASCII range
    let mut glyph_infos = HashMap::new();
    
    let mut max_height = 0;
    let mut total_width = 0;

    let glyphs: Vec<_> = chars.iter()
        .map(|&c| font.glyph(c).scaled(scale).positioned(point(0.0, 0.0)))
        .collect();

    for glyph in &glyphs {
        if let Some(bb) = glyph.pixel_bounding_box() {
            max_height = max_height.max(bb.height());
            total_width += bb.width();
        }
    }

    let mut atlas = image::RgbaImage::new(total_width as u32, max_height as u32);
    let mut x_offset = 0;

    for (i, glyph) in glyphs.iter().enumerate() {
        if let Some(bb) = glyph.pixel_bounding_box() {
            
            let glyph_width = bb.width();
            let glyph_height = bb.height();

            let mut glyph_bitmap = image::RgbaImage::new(glyph_width as u32, glyph_height as u32);
            glyph.draw(|x, y, v| {
                let intensity = (v * 255.0) as u8;
                glyph_bitmap.put_pixel(x, y, image::Rgba([255, 255, 255, intensity]));
            });

            image::imageops::overlay(&mut atlas, &glyph_bitmap, x_offset as i64, 0);

            glyph_infos.insert(chars[i], Glyph {
                tex_coords: [
                    x_offset as f32 / atlas.width() as f32,
                    0.0,
                    (x_offset + glyph_width) as f32 / atlas.width() as f32,
                    glyph_height as f32 / atlas.height() as f32,
                ],
                size: [glyph_width as f32, glyph_height as f32],
                bearing: [bb.min.x as f32, bb.min.y as f32],
                advance: glyph.unpositioned().h_metrics().advance_width,
            });

            x_offset += glyph_width;
            debug!("Glyph: {}, Width: {}", chars[i], glyph_width)
        }
    }
    atlas.save("atlas_example.png").expect("Failed to save atlas");
    
    for (key, glyph) in &glyph_infos {
        debug!("{}, {:?}", key, glyph.size)
    }

    // Rust drops the space character for some reason so we're putting it back in here
    let space_glyph = font.glyph(' ').scaled(scale);
    let space_advance = space_glyph.h_metrics().advance_width;
    glyph_infos.insert(' ', Glyph {
        tex_coords: [0.0, 0.0, 0.0, 0.0],   // No texture needed
        size: [0.0, 0.0],                   // No size
        bearing: [0.0, 0.0],                // No bearing
        advance: space_advance,             // Correct spacing
    });
    
    (atlas, glyph_infos)
}

/// OpenGL vertex struct, differs from main one in that it has texture coordinates
#[derive(Debug, Copy, Clone)]
pub struct TextureVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl TextureVertex {
    fn new(position: [f32; 2], tex_coords: [f32;2]) -> Self {
        TextureVertex {
            position: [position[0], position[1], 0.0],
            tex_coords: tex_coords,
        }
    }
}
implement_vertex!(TextureVertex, position, tex_coords);

/// Creates texture from font atlas for OpenGL
pub fn create_texture_atlas(display: &Display<WindowSurface>, atlas: image::RgbaImage) -> Texture2d {
    let image_dimensions = atlas.dimensions();
    let raw_data = atlas.clone().into_raw(); // Convert to Vec<u8>

    // Validates data size 
    let expected_size = (image_dimensions.0 * image_dimensions.1 * 4) as usize; // 4 bytes per pixel (RGBA)
    assert_eq!(
        raw_data.len(),
        expected_size,
        "Texture data size mismatch: expected {} bytes, found {} bytes",
        expected_size,
        raw_data.len()
    );
    debug!("Texture sample: {:?}", atlas.iter().take(20).collect::<Vec<_>>());
    // Checks it's in the correct format
    let raw = RawImage2d::from_raw_rgba_reversed(&raw_data, image_dimensions);
    let texture = Texture2d::with_format(
            display, 
            raw,
        glium::texture::UncompressedFloatFormat::U8U8U8U8,
        glium::texture::MipmapsOption::NoMipmap,
    ).expect("Failed to create texture atlas");
    return texture;
}

// Struct for DATACOM to display text.
pub struct TextDisplay {
    content: String,
    glyph_map: Arc<HashMap<char, Glyph>>,
    texture_ref: Arc<Texture2d>,
    x_start: f32,
    y_start: f32,
    color: Rgba,
}

impl TextDisplay {
    pub fn new(content: String, glyph_map: Arc<HashMap<char, Glyph>>, texture_ref: Arc<Texture2d>, x_start: f32, y_start: f32, color: Rgba) -> Self {
        TextDisplay {
            content: content,
            glyph_map: glyph_map,
            texture_ref: texture_ref, 
            x_start: x_start,
            y_start: y_start,
            color: color,
        }
    }

    /// Function to change text in string.
    pub fn change_text(&mut self, new_string: String) {
        self.content = new_string;
    }
}

impl Draw2 for TextDisplay {
    fn draw(&self, gui: &crate::dc::GuiContainer, context: &crate::dc::RenderContext, target: &mut glium::Frame) {
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut cursor_x = 0.0;
        // debug!("Drawing text");
        for (i, c) in (&self).content.chars().enumerate() {
            if let Some(glyph) = self.glyph_map.get(&c) {
                let x0 = cursor_x + glyph.bearing[0];
                let y0 = 0.0 - glyph.bearing[1];
                let x1 = x0 + glyph.size[0];
                let y1 = y0 - glyph.size[1];

                let tex_coords = glyph.tex_coords;
                let base = (i * 4) as u16;
                vertices.push(TextureVertex::new([x0, y0], [tex_coords[0], 1.0 - tex_coords[1]]));
                vertices.push(TextureVertex::new([x1, y0], [tex_coords[2], 1.0 - tex_coords[1]]));
                vertices.push(TextureVertex::new([x1, y1], [tex_coords[2], 1.0 - tex_coords[3]]));
                vertices.push(TextureVertex::new([x0, y1], [tex_coords[0], 1.0 - tex_coords[3]]));

                indices.extend_from_slice(&[
                    base, base+1, base+2, 
                    base, base+2, base+3]);

                cursor_x += glyph.advance;
            }
        }

        // debug!("Text vertices: {:?}", vertices);
        let vertex_buffer = glium::VertexBuffer::new(&gui.display, &vertices).unwrap();
        let index_buffer = glium::IndexBuffer::new(&gui.display, glium::index::PrimitiveType::TrianglesList, &indices).unwrap();

        let draw_params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfLess,
                write: false,
                ..Default::default()
            },
            blend: glium::Blend {
                color: glium::BlendingFunction::Addition {
                    source: glium::LinearBlendingFactor::SourceAlpha,
                    destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
                },
                alpha: glium::BlendingFunction::Addition {
                    source: glium::LinearBlendingFactor::One,
                    destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
                },
                ..Default::default()
            },
            ..Default::default()
        };
        
        // let uniforms = glium::uniform! { tex: &*self.texture_ref, color_obj: uniformify_vec4(dc::green_vec()) };
        let screen_size = [gui.display.get_framebuffer_dimensions().0 as f32, gui.display.get_framebuffer_dimensions().1 as f32];

        let model = na::Matrix4::new_translation(&na::Vector3::new(self.x_start, self.y_start, 0.0));
        let projection = na::Matrix4::new_orthographic(0.0, screen_size[0], 0.0, screen_size[1], 0.1, 1000.0);

        let uniforms = glium::uniform! {
            tex: &*self.texture_ref,
            color_obj: uniformify_vec4(self.color),
            model: uniformify_mat4(model),
            projection: uniformify_mat4(projection),
            screen_size: screen_size,
        };
        

        target.draw(&vertex_buffer, &index_buffer, &gui.text_shaders, &uniforms, &draw_params).unwrap();
    }
}