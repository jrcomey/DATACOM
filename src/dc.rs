extern crate glium;
use std::{fs::read_to_string, rc::Rc, sync::{Arc, RwLock}, time::Instant};
use crate::scenes_and_entities;
use glium::{glutin::{self, surface::WindowSurface}, Display, Surface};
use glium::winit::{self, window, 
    event::{
        MouseScrollDelta,
    }
};
use log::{debug, info};
use std::fmt::Error;
use crate::scenes_and_entities::Scene;

pub type Rgba = na::SVector<f32, 4>;

// #####################

/* VIEWPORT DEFINITION */

/* 
    This is a general viewport, the basic building block of the data visualizer program. 
    It has some height and width, and a root location. It also contains a pointer to the content that is drawn within it.
    When drawn, it draws a box outline around its position, and then draws its dependent content 
*/

// #####################

/* VIEWPORT STRUCT */

pub struct Viewport {
    pub color: Rgba,                  // Color of viewport
    pub is_active: bool,              // Active status of viewport
    pub root: na::Point2<f64>,        // Upper left window coordinate
    pub height: f64,                  // Height of window from top -> down side   (RANGE 0 -> 2)
    pub width: f64,                   // Width of window from left -> right side  (RANGE 0 -> 2)
    pub content: Arc<RwLock<Scene>>,  // Pointer to content to be drawn           
    pub context: RenderContext,       // Local Viewport Render Context
    pub camera: Camera                // Camera struct
}

impl Viewport {

    // Creates a new viewport on the screen, initialized with an included camera. 
    pub fn new_with_camera(root: na::Point2<f64>, height: f64, width: f64, content: Arc<RwLock<Scene>>, camera_position: na::Point3<f64>, camera_target: na::Point3<f64>) -> Viewport {
        Viewport {
            color: cyan_vec(),
            is_active: false, 
            root: root,
            height: height,
            width: width,
            content: content,
            context: RenderContext::new(
                na::Matrix4::look_at_rh(                // Camera Position
                    &na::convert(camera_position),   
                    &na::convert(camera_target), 
                    &na::Vector3::z_axis()
                ), 
                na::Matrix4::new_perspective((3.0/2.0) as f32, 3.141592 / 3.0, 0.1, 1024.0E6),
                xy_translation((root[0]+width/2.0) as f32, (root[1]-height/2.0) as f32),
                glium::Rect{
                    left: 0, 
                    bottom: 0, 
                    width: 0, 
                    height: 0
                }
            ),
            camera: Camera::new(camera_position, camera_target, CameraMode::Tracking)
        }
    }

    pub fn update_all_graphical_elements(&mut self, target: &glium::Frame) {
        let (width, height) = target.get_dimensions();

        // let bounds = na::base::Vector4::<f32>::new(                                                  // Bounds
        //     (self.root[0]) as f32,   // Left X
        //     (self.root[0]+self.width) as f32, // Right X
        //     (self.root[1]-self.height) as f32, // Bottom Y 
        //     (self.root[1]) as f32    // Top Y
        // );

        let bounds = na::base::Vector4::<f32>::new(                                                  // Bounds
            (self.root[0]) as f32,   // Left X
            (self.root[0]+self.width) as f32, // Right X
            (self.root[1]-self.height) as f32, // Bottom Y 
            (self.root[1]) as f32    // Top Y
        );



        let boxxy = glium::Rect{
            left: (((bounds[0]+1.0)/2.0) * width as f32) as u32,
            bottom: (((bounds[2]+1.0)/2.0) * height as f32) as u32,
            width: ((bounds[1]-bounds[0])/2.0*width as f32) as u32,
            height: ((bounds[3]-bounds[2])/2.0*height as f32) as u32};
        self.context.update_render_context(width, height, boxxy);
        self.camera.update_camera(self.content.clone());
        self.context.update_view(self.camera.get_camera_mat());
    }

    pub fn set_active(&mut self) {
        self.is_active = true;
        self.set_color(green_vec());
    }

    /// Sets status of the viewport to inactive.
    pub fn set_inactive(&mut self) {
        self.is_active = false;
        self.set_color(red_vec());
    }

    /// Sets a new color for the viewport box
    pub fn set_color(&mut self, new_color: na::Vector4<f32>) {
        self.color = new_color;
    }

    /// Checks to see if a queried position is within the viewport box or not
    pub fn in_viewport(&mut self, gui: &GuiContainer, query_x_position: &u32, query_y_position: &u32) -> bool {
        // self.context.is_within_pixel_bounds(query_x_position, query_y_position)
        let (viewport_width, viewport_height) = gui.display.get_framebuffer_dimensions();

        let root_x_pixels = (((self.root[0]+1.0)/2.0) * viewport_width as f64) as u32;
        let width_pixels = (((self.width)/2.0) * viewport_width as f64) as u32;
        let root_y_pixels = (((-self.root[1]+1.0)/2.0) * viewport_height as f64) as u32;
        let height_pixels = (((self.height)/2.0) * viewport_height as f64) as u32;
        if query_x_position >= &root_x_pixels 
        && query_x_position <= &(width_pixels+root_x_pixels)
        && query_y_position >= &root_y_pixels
        && query_y_position <= &(height_pixels + root_y_pixels) {
            return true;
        }
        else {
            return false;
        }
    }

    
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            color: cyan_vec(),
            is_active: false, 
            root: na::OPoint::origin(),
            height: 0.0,
            width: 0.0,
            content: Arc::new(RwLock::new(Scene::new())),
            context:RenderContext::new(na::Matrix4::zeros(), na::Matrix4::zeros(), na::Matrix4::zeros(), glium::Rect { left: 0, bottom: 0, width: 0, height: 0 }), 
            camera: Camera { ..Default::default() }
        }
    }
}

impl Draw for Viewport {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame) {
        
        // Create uniforms
        let uniforms = glium::uniform! {
            model: uniformify_mat4(eye4()),              // Identity matrix for M, not moving anywhere
            view: uniformify_mat4(eye4()),               // Identity matrix for V, should be viewed dead on
            perspective: uniformify_mat4(eye4()),        // Identity matrix for P, should not have perspective
            color_obj: uniformify_vec4(self.color),      // Use set color
            vp: uniformify_mat4(eye4()),                 // Identity matrix for post processing move. Viewport is stationary relative to itself!
            bounds: uniformify_vec4(full_range_vec()),   // Viewports should be able to take up the whole screen
        };

        // Define Positions, if necessary

        // Positions for the viewport are the corners, linked ina  circular pattern.
        let positions = vec![
            Vertex::newtwo(self.root[0], self.root[1]),
            Vertex::newtwo(self.root[0]+self.width, self.root[1]),
            Vertex::newtwo(self.root[0]+self.width, self.root[1]-self.height),
            Vertex::newtwo(self.root[0], self.root[1]-self.height)
        ];

        // Buffer definitions
        let index_list: [u16; 4] = [0, 1, 2, 3];
        let positions = glium::VertexBuffer::new(&gui.display, &positions).unwrap();
        let indices = glium::IndexBuffer::new(&gui.display, glium::index::PrimitiveType::LineLoop, &index_list).unwrap();

        // Define draw parameters
        let params = glium::DrawParameters {
            polygon_mode: glium::draw_parameters::PolygonMode::Line,
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };

        // Call glium draw function
        target.draw(
            &positions, 
            &indices, 
            &gui.program, 
            &uniforms, 
            &params)
                .unwrap();

        // Draw Dependents  
        self.content.read().unwrap().draw(&gui, &self.context, target); // Draw dependent content
    }
}

pub trait CameraControl {
    fn move_camera(&mut self, delta_camera: na::Vector3<f64>);
    fn change_camera_position(&mut self, new_camera_position: na::Point3<f64>);
    fn zoom(&mut self, mouse_delta: MouseScrollDelta);
    fn get_camera_radius_vector(&self) -> na::Vector3<f64>;
    fn orbit(&mut self,rotation_theta_degree: f64, rotation_phi_degree: f64, up_direction: na::Vector3<f64>);
    fn update_camera(&mut self, scene: Arc<RwLock<Scene>>);
    fn get_target_id(&self)->Result<u64, Error>;
    fn set_target_id(&mut self, new_target_id: u64);
    fn switch_mode(&mut self, new_mode: CameraMode);
    fn advance_mode(&mut self);
}

impl CameraControl for Viewport {
        
    fn move_camera(&mut self, delta_camera: na::Vector3<f64>){
        self.camera.move_camera(delta_camera);
    }

    fn change_camera_position(&mut self, new_camera_position: na::Point3<f64>) {
        self.camera.change_camera_position(new_camera_position);
    }

    /// Zooms in by a perscribed magnitude
    fn zoom(&mut self, mouse_delta: MouseScrollDelta) {
        self.camera.zoom(mouse_delta);
    }

    fn get_camera_radius_vector(&self) -> na::Vector3<f64> {
        todo!()
    }

    fn orbit(&mut self,rotation_theta_degree: f64, rotation_phi_degree: f64, up_direction: na::Vector3<f64>) {
        self.camera.orbit(rotation_theta_degree, rotation_phi_degree, up_direction);
    }

    fn update_camera(&mut self, scene: Arc<RwLock<Scene>>) {
        self.camera.update_camera(scene);
    }

    fn get_target_id(&self)->Result<u64, Error> {
        Ok(self.camera.get_target_id()?)
    }

    fn set_target_id(&mut self, new_target_id: u64) {
        self.camera.set_target_id(new_target_id);
    }

    fn switch_mode(&mut self, new_mode: CameraMode) {
        self.camera.switch_mode(new_mode);
    }

    fn advance_mode(&mut self) {
        self.camera.advance_mode();
    }

}


// #####################

pub struct RenderContext {
    pub view: na::Matrix4<f32>,                         // Camera view of viewport
    pub perspective: na::Matrix4<f32>,                  // Perspective Matrix,
    pub viewport_shift: na::Matrix4<f32>,               // Postprocessing viewport shift to keep in bounds
    pub pixel_bounds: glium::Rect,                      // Scissors Perspective to restricted pixel bounds
}

impl RenderContext {

    pub fn new (view: na::Matrix4<f32>, perspective: na::Matrix4<f32>, viewport_shift: na::Matrix4<f32>, pixel_bounds: glium::Rect) -> RenderContext {
        RenderContext{
            view: view,
            perspective: perspective,
            viewport_shift: viewport_shift,
            pixel_bounds: pixel_bounds
        }
    }

    pub fn new_null() -> Self {
        RenderContext { 
            view: na::Matrix4::<f32>::zeros(), 
            perspective: na::Matrix4::<f32>::zeros(), 
            viewport_shift: na::Matrix4::<f32>::zeros(), 
            pixel_bounds: glium::Rect { left: 0, bottom: 0, width: 0, height: 0 } 
        }
    }

    pub fn update_render_context(&mut self, new_pixel_width: u32, new_pixel_height: u32, new_box: glium::Rect) {
        self.update_perspective(na::base::Matrix4::new_perspective(new_pixel_width as f32/new_pixel_height as f32, 3.141592 / 3.0, 0.1 , 1024.0E6));
        // self.update_perspective(na::base::Matrix4::new_orthographic((((self.mvp.bounds[0]+1.0)/2.0) * width as f32), right, bottom, top, znear, zfar))
        self.update_viewport_pixel_bounds(new_box);
    }

    pub fn update_view(&mut self, new_view: na::Matrix4<f32>) {
        self.view = new_view;
    }

    pub fn update_perspective(&mut self, new_perspective: na::Matrix4<f32>) {
        self. perspective = new_perspective;
    }

    pub fn update_viewport_pixel_bounds(&mut self, new_bounds: glium::Rect) {
        self.pixel_bounds = new_bounds;
    }

}

// #####################

pub enum CameraMode {
    Static,
    Following,
    Tracking,
    Orbit,
}

impl CameraMode {
    pub fn advance_mode(&mut self) -> CameraMode {
        match &self {
            &CameraMode::Static => {
                debug!("NOW FOLLOWING");
                CameraMode::Following
            },
            &CameraMode::Following => {
                debug!("NOW TRACKING");
                CameraMode::Tracking},
            &CameraMode::Tracking => {debug!("NOW ORBIT");
                CameraMode::Orbit},
            &CameraMode::Orbit => {
                debug!("NOW STATIC");
                CameraMode::Static}
        }
    }
}

pub struct Camera {
    pub camera_position: na::Point3<f64>,               // Camera position
    pub camera_target: na::Point3<f64>,                 // Position of the target the camera is looking at
    camera_mode: CameraMode,                            // Enum describing camera mode
    relative_up_direction: na::Vector3<f64>,            // Relative up direction (Z-Axis default)
    target_id: Option<u64>
}

impl Camera {

    pub fn new(camera_position: na::Point3<f64>, camera_target: na::Point3<f64>, mode: CameraMode) -> Self{
        Camera {
            camera_position: camera_position,
            camera_target: camera_target,
            camera_mode: mode,
            ..Default::default()
        }
    }

    pub fn get_camera_mat(&self) -> na::Matrix4<f32> {
        na::Matrix4::<f32>::look_at_rh(
            &na::convert(self.camera_position),
            &na::convert(self.camera_target), 
            &na::Vector3::z_axis()
        )
    }

    pub fn set_new_target(&mut self, new_target: na::Point3<f64>) {
        self.camera_target = new_target;
    }

    pub fn set_new_position(&mut self, new_position: na::Point3<f64>) {
        self.camera_position = new_position;
    }

    pub fn update_camera2(&mut self, scene: Arc<RwLock<scenes_and_entities::Scene>>) {
        match &self.camera_mode {
            CameraMode::Static => {},
            CameraMode::Following => {
                let delta_pos = scene.write().unwrap().get_entity(self.target_id.unwrap() as usize).expect("Out of bounds!").get_position() - self.camera_target;
                self.set_new_target(self.camera_target+delta_pos);      // This solves a borrow
                self.move_camera(delta_pos);
            }
            CameraMode::Tracking => {
                let delta_pos = scene.write().unwrap().get_entity(self.target_id.unwrap() as usize).expect("Out of bounds!").get_position() - self.camera_target;
                self.set_new_target(self.camera_target+delta_pos);
            }
            _ => {}
        }
    }
}

impl CameraControl for Camera {
    fn move_camera(&mut self, delta_camera: na::Vector3<f64>) {
        self.camera_position = self.camera_position + delta_camera;
    }

    fn zoom(&mut self, mouse_delta: MouseScrollDelta) {
        let zoom_magnitude = match mouse_delta {
            MouseScrollDelta::LineDelta(_, mouse_main) => {
                mouse_main as f64
            },
            _ => {
                0.0
            }
        };

        debug!("ZOOM FACTOR: {}", zoom_magnitude);

        let r_bar = self.camera_position - self.camera_target;
        let r_hat = r_bar / r_bar.magnitude();
        self.camera_position = self.camera_position - zoom_magnitude * r_hat * r_bar.magnitude()/100.0;
    }

    fn change_camera_position(&mut self, new_camera_position: na::Point3<f64>) {
        self.camera_position = new_camera_position;
    }

    fn get_camera_radius_vector(&self) -> na::Vector3<f64> {
        self.camera_position - self.camera_target
    }

    fn orbit(&mut self,rotation_theta_degree: f64, rotation_phi_degree: f64, up_direction: na::Vector3<f64>) {
        
        // Convert orbit command to radians and structure as a vector
        let theta = rotation_theta_degree * std::f64::consts::PI / 180.0;
        let phi = rotation_phi_degree * std::f64::consts::PI / 180.0;
        let delta_vector_spherical = na::base::Vector3::new(
            0.0,
            theta,
            phi
        );

        // Normalize camera position relative to target and convert to spherical coordinates
        let spherical_transform = na::base::Matrix3::new(
            theta.sin()*phi.cos(), theta.cos()*phi.sin(), -phi.sin(),
            theta.sin()*phi.sin(), theta.cos()*phi.sin(), phi.cos(),
            theta.cos(), -theta.sin(), 0.0,
        );

        let normalized_camera_position = self.camera_position - self.camera_target;

        // Convert to spherical:
        // rho:     sqrt(x^2 + y^2 + z^2)
        // theta:   atan(y/x)
        // phi:     arccos(z / sqrt(x^2 + y^2 + z^2))
        let mut normalized_spherical_camera_position = na::Vector3::new(
            (normalized_camera_position.x.powf(2.0)+normalized_camera_position.y.powf(2.0)+normalized_camera_position.z.powf(2.0)).sqrt(), 
            normalized_camera_position.y.atan2(normalized_camera_position.x), 
            (normalized_camera_position.z / (normalized_camera_position.x.powf(2.0)+normalized_camera_position.y.powf(2.0)+normalized_camera_position.z.powf(2.0)).sqrt()).acos()
        );

        // Add delta vector
        normalized_spherical_camera_position += delta_vector_spherical;


        // Convert new camera position back to cartesian coordinates and de-normalize
        // x = rho * sin(theta) * cos(phi)
        self.camera_position[0] = normalized_spherical_camera_position[0]*normalized_spherical_camera_position[1].cos()*normalized_spherical_camera_position[2].sin() + self.camera_target[0];
        // y = rho * sin(theta) * sin(theta)
        self.camera_position[1] = normalized_spherical_camera_position[0]*normalized_spherical_camera_position[1].sin()*normalized_spherical_camera_position[2].sin() + self.camera_target[1];
        // z = rho * cos(phi)
        self.camera_position[2] = normalized_spherical_camera_position[0] * normalized_spherical_camera_position[2].cos() + self.camera_target[2];
        
    }

    fn update_camera(&mut self, scene: Arc<RwLock<Scene>>) {
        self.update_camera2(scene);
    }

    fn get_target_id(&self)->Result<u64, Error> {
        match self.target_id {
            Some(id) => Ok(id),
            None => Err(Error)
        }
    }

    fn set_target_id(&mut self, new_target_id: u64) {
        self.target_id = Some(new_target_id);
    }

    fn switch_mode(&mut self, new_mode: CameraMode) {
        self.camera_mode = new_mode;
    }

    fn advance_mode(&mut self) {
        self.camera_mode = self.camera_mode.advance_mode();
    }
    
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            camera_position: na::Point3::<f64>::new(10.0, 10.0, 10.0),
            camera_target: na::Point3::origin(),
            camera_mode: CameraMode::Following,
            relative_up_direction: na::Vector3::z(),
            target_id: Some(0),
        }
    }
}

pub trait Draw {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame);
}

pub struct Text {
    content: String,
    font_size: u8,
    
}

pub enum TextJustification {
    Left,
    Center,
    Right,
}




























































// ################################################################################################

// #####################

/* NULL CONTENT */

/* 
    This is a null item for content. Intended for test purposes, or to load something blank.
*/

// #####################

/* NULL STRUCT */
pub struct null_content {
    null: i8
}

/* NULL SPECIFIC FUNCTIONALITY */
impl null_content {
    pub fn new() -> null_content {
        null_content {
            null: 0
        }
    }
}

impl Draw for null_content {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame){

    }
}
// ################################################################################################

/* Display Engine Structs & Functions */

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress, // how wide the vertex is
            step_mode: wgpu::VertexStepMode::Vertex, // whether each buffer element is per-vertex or per-instance
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
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// Struct for information containing program-level OpenGL elements
pub struct GuiContainer {
    pub display: glium::Display<WindowSurface>,
    pub program: glium::Program,
    pub text_shaders: glium::Program,
    pub window: window::Window,
}

// Functionality for OpenGL struct
impl GuiContainer {
    pub fn new(display: glium::Display<WindowSurface>, program: glium::Program, text_shaders: glium::Program, window: window::Window) -> Self {
        GuiContainer { display: display, program: program, text_shaders: text_shaders, window: window}
    }

    pub fn init_opengl(event_loop: &glium::winit::event_loop::EventLoop<()>) -> Self {
        use glium::{backend::glutin, Surface};



        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_title("DATACOM - Data Communications and Visual Terminal")
            .build(event_loop);


        // Vertex Shader
        let vertex_shader_src = read_to_string("src/shaders/vertex_shader.glsl").unwrap();

        // Fragment Shader
        let fragment_shader_src = read_to_string("src/shaders/fragment_shader.glsl").unwrap();

        let text_vertex_shader_src = read_to_string("src/shaders/text_vertex_shader.glsl").unwrap();

        let text_fragment_shader_src = read_to_string("src/shaders/text_fragment_shader.glsl").unwrap();
        let program = glium::Program::from_source(&display, &vertex_shader_src,& fragment_shader_src, None).unwrap();
        let text_shader = glium::Program::from_source(&display, &text_vertex_shader_src, &text_fragment_shader_src, None).unwrap();

        return GuiContainer::new(display, program, text_shader, window);
    }
}
// ################################################################################################

/* UTILITY FUNCTIONS */

// 4x1 vector for green color
pub fn green_vec() -> Rgba {
    na::base::Vector4::<f32>::new(0.0, 1.0, 0.0, 1.0)
}

/// Cyan
pub fn cyan_vec() -> Rgba {
    na::base::Vector4::<f32>::new(0.0, 100.0/255.0, 100.0/255.0, 0.0)
}

/// Red
pub fn red_vec() -> Rgba {
    na::base::Vector4::<f32>::new(1.0, 0.0, 0.0, 0.0)
}

/// Blue
pub fn blue_vec() -> Rgba {
    na::base::Vector4::<f32>::new(0.0, 0.0, 1.0, 0.0)
}

/// White
pub fn white_vec() -> Rgba {
    na::base::Vector4::<f32>::new(1.0, 1.0, 1.0, 1.0)
}

/// 4x4 identity matrix
pub fn eye4() -> na::base::Matrix4<f32> {
    na::base::Matrix4::identity()
}

/// 3x1 Vector of zeros
pub fn null3() -> na::base::Vector3<f64> {
    na::base::Vector3::new(0.0, 0.0, 0.0)
}

/// Data type conversion function to make it usable for OpenGL
pub fn uniformify_mat4 (target: na::base::Matrix4<f32>) -> [[f32; 4]; 4] {
    *target.as_ref()
}

/// Data type conversion function to make it usable for OpenGL
pub fn uniformify_vec4 (target: Rgba) -> [f32; 4] {
    *target.as_ref()
}

/// translation matrix for XY plane
pub fn xy_translation(x: f32, y: f32) -> na::base::Matrix4<f32> {
    na::Matrix4::new_translation(
        &na::base::Vector3::<f32>::new(x, y, 0.0)
    )
}

/// bounds for full range of screen
pub fn full_range() -> [f32; 4] {
    [-1.0, 1.0, -1.0, 1.0]
}

/// bounds for full range in vec4 format
pub fn full_range_vec() -> Rgba {
    na::base::Vector4::new(
        -1.0,
        1.0, 
        -1.0, 
        1.0
    )
}

/// Quick function for RGBA nalgebra vector
pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Rgba {
    na::base::Vector4::new(
        r,
        g, 
        b, 
        a
    )
}

/// Enum for draw type
pub enum DrawType {
    Full,
    RotationOnly,
    PositionOnly,
    NoDraw,
}

/// Draw type functionality
impl DrawType {
    pub fn change_draw_type(kind: &str) -> DrawType {
        match kind {
            "Full" => DrawType::Full,
            "RotationOnly" => DrawType::RotationOnly,
            "PositionOnly" => DrawType::PositionOnly,
            "NoDraw" => DrawType::NoDraw,
            _ => panic!("DrawType switch error! Requested type {} does not exist. Exiting program...", kind),
        }
    }
}