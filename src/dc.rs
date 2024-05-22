extern crate glium;
use std::{rc::Rc, sync::{Arc, RwLock}, time::Instant};
use glium::{glutin::{self, window, event::MouseScrollDelta}, Surface, debug};

// #####################

/* VIEWPORT DEFINITION */

/* 
    This is a general viewport, the basic building block of the data visualizer program. 
    It has some height and width, and a root location. It also contains a pointer to the content that is drawn within it.
    When drawn, it draws a box outline around its position, and then draws its dependent content 
*/

// #####################

/* VIEWPORT STRUCT */

pub struct Twoport {
    pub color: na::base::Vector4<f32>,                  // Color of viewport
    pub is_active: bool,                                // Active status of viewport
    pub root: na::Point2<f64>,                          // Upper left window coordinate
    pub height: f64,                                    // Height of window from top -> down side   (RANGE 0 -> 2)
    pub width: f64,                                     // Width of window from left -> right side  (RANGE 0 -> 2)
    pub content: Arc<RwLock<dyn Draw2>>,                        // Pointer to content to be drawn           
    pub context: RenderContext,                         // Local Viewport Render Context
    pub camera_position: na::Point3<f64>,               // Camera position
    pub camera_target: na::Point3<f64>,                 // Position of the target the camera is looking at
}

impl Twoport {

    // Creates a new viewport on the screen, initialized with an included camera. 
    pub fn new_with_camera(root: na::Point2<f64>, height: f64, width: f64, content: Arc<RwLock<dyn Draw2>>, camera_position: na::Point3<f64>, camera_target: na::Point3<f64>) -> Twoport {
        Twoport {
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
                na::Matrix4::new_perspective((3.0/2.0) as f32, 3.141592 / 3.0, 0.1, 1024.0),
                xy_translation((root[0]+width/2.0) as f32, (root[1]-height/2.0) as f32),
                glium::Rect{
                    left: 0, 
                    bottom: 0, 
                    width: 0, 
                    height: 0
                }
            ),
            camera_position: camera_position,
            camera_target: camera_target
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
    }

    pub fn move_camera(&mut self, delta_camera: na::Vector3<f64>){
        self.camera_position = self.camera_position + delta_camera;
        self.context.update_view(na::Matrix4::look_at_rh(                // Camera Position
            &na::convert(self.camera_position),   
            &na::convert(self.camera_target), 
            &na::Vector3::z_axis()
            )
        )
    }

    pub fn change_camera_position(&mut self, new_camera_position: na::Point3<f64>) {
        self.camera_position = new_camera_position;
        self.context.update_view(na::Matrix4::look_at_rh(                // Camera Position
            &na::convert(self.camera_position),   
            &na::convert(self.camera_target), 
            &na::Vector3::z_axis()
            )
        )
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

    /// Zooms in by a perscribed magnitude
    pub fn zoom(&mut self, mouse_delta: MouseScrollDelta) {

        let zoom_magnitude = match mouse_delta {
            MouseScrollDelta::LineDelta(_, mouse_main) => {
                mouse_main as f64
            },
            _ => {0.0}
        };

        debug!("ZOOM FACTOR: {}", zoom_magnitude);

        let r_bar = self.camera_position - self.camera_target;
        let r_hat = r_bar / r_bar.magnitude();
        self.camera_position = self.camera_position - zoom_magnitude * r_hat;
        self.context.update_view(na::Matrix4::look_at_rh(
            &na::convert(self.camera_position),   
            &na::convert(self.camera_target), 
            &na::Vector3::z_axis()
            )
        )
    }

    fn get_camera_radius_vector(&self) -> na::Vector3<f64> {
        self.camera_position - self.camera_target
    }

    pub fn orbit(&mut self,rotation_theta_degree: f64, rotation_phi_degree: f64, up_direction: na::Vector3<f64>) {

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
        


        // let r_bar = self.get_camera_radius_vector();
        // self.camera_position = self.camera_target + r_bar + spherical_transform*delta_vector_spherical;
        self.context.update_view(na::Matrix4::look_at_rh(
            &na::convert(self.camera_position),   
            &na::convert(self.camera_target), 
            &na::Vector3::z_axis()
            )
        );
    }
}

impl Draw2 for Twoport {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame) {
        // println!("Drawing window...");

        // Create uniforms
        let uniforms = glium::uniform! {
            model: uniformifyMat4(eye4()),              // Identity matrix for M, not moving anywhere
            view: uniformifyMat4(eye4()),               // Identity matrix for V, should be viewed dead on
            perspective: uniformifyMat4(eye4()),        // Identity matrix for P, should not have perspective
            color_obj: uniformifyVec4(self.color),      // Use set color
            vp: uniformifyMat4(eye4()),                 // Identity matrix for post processing move. Viewport is stationary relative to itself!
            bounds: uniformifyVec4(full_range_vec()),   // Viewports should be able to take up the whole screen
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

    pub fn update_render_context(&mut self, new_pixel_width: u32, new_pixel_height: u32, new_box: glium::Rect) {
        self.update_perspective(na::base::Matrix4::new_perspective(new_pixel_width as f32/new_pixel_height as f32, 3.141592 / 3.0, 0.1 , 1024.0));
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

// pub struct Camera {
//     pub camera_position: na::Point3<f64>,               // Camera position
//     pub camera_target: na::Point3<f64>,                 // Position of the target the camera is looking at
// }

// impl Camera {

//     pub fn new(camera_position: na::Point3<f64>, camera_target: na::Point3<f64>) {

//     }

//     pub fn move_camera(&mut self, delta_camera: na::Vector3<f64>){
//         self.camera_position = self.camera_position + delta_camera;
//         self.context.update_view(na::Matrix4::look_at_rh(                // Camera Position
//             &na::convert(self.camera_position),   
//             &na::convert(self.camera_target), 
//             &na::Vector3::z_axis()
//             )
//         )
//     }
// }

pub trait Draw2 {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame);
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

/* NULL DRAW TRAIT */
impl Draw for null_content {
    fn draw(&self, gui: &GuiContainer, mvp: &MVPetal, target: &mut glium::Frame) {
        ;
        // println!("Drawing null content...");
    }

    fn draw_absolute(&self, gui: &GuiContainer, mvp: &MVPetal, target: &mut glium::Frame) {
        error!("Not implemented!");
    }
}

impl Draw2 for null_content {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame){
        ;
    }
}
// ################################################################################################

/* Display Engine Structs & Functions */

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f64; 3],
}
glium::implement_vertex!(Vertex, position);

impl Vertex {
    pub fn newtwo(x: f64, y: f64) -> Vertex {
        Vertex { position: [x, y, 0.0] }
    }

    pub fn new(x: f64, y: f64, z: f64) -> Vertex {
        Vertex { position: [x, y, z] }
    }
}


#[derive(Copy, Clone)]
pub struct Normal {
    normal: [f64; 3],
}

impl Normal {
    pub fn new(x: f64, y: f64, z: f64) -> Normal {
        Normal { normal: [x, y, z] }
    }
}
glium::implement_vertex!(Normal, normal);

// Struct for information containing program-level OpenGL elements
pub struct GuiContainer {
    pub display: glium::Display,
    pub program: glium::Program
}

// Functionality for OpenGL struct
impl GuiContainer {
    fn new(display: glium::Display, program: glium::Program) -> GuiContainer {
        GuiContainer { display: display, program: program,}
    }

    pub fn init_opengl(event_loop: &glutin::event_loop::EventLoop<()>) -> GuiContainer {
        use glium::{glutin, Surface};

        // let event_loop = glutin::event_loop::EventLoop::new();
        let window_builder = glutin::window::WindowBuilder::new();
        let context_builder = glutin::ContextBuilder::new().with_depth_buffer(24);
        let display = glium::Display::new(window_builder, context_builder, &event_loop).unwrap();

        // Vertex Shader
        let vertex_shader_src = r#"
            #version 140
            in vec3 position;
            uniform mat4 model;
            uniform mat4 view;
            uniform mat4 perspective;
            uniform mat4 vp;

            void main() {
                gl_Position = vp * perspective * view * model * vec4(position, 1.0);
            }
        "#;

        // Fragment Shader
        let fragment_shader_src = r#"
            #version 140    
            out vec4 color;

            uniform vec4 color_obj;

            void main() {
                color = vec4(color_obj);
            }
        "#;

        let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

        return GuiContainer::new(display, program);
    }
}

// Draw trait for drawing to the interface
pub trait Draw {
    fn draw(&self, gui: &GuiContainer, mvp: &MVPetal, target: &mut glium::Frame);
    fn draw_absolute(&self, gui: &GuiContainer, mvp: &MVPetal, target: &mut glium::Frame);
}

// Container struct to reduce draw arguements. MVP matricies and others.
pub struct MVPetal {
    pub model: na::Matrix4<f32>,                        // Model Matrix                     (Position of the object in world)
    pub view: na::Matrix4<f32>,                         // View Matrix                      (Camera position)
    pub perspective: na::Matrix4<f32>,                  // Perspective Matrix               (Adds perspective)
    pub vp: na::Matrix4<f32>,                           // Viewport position transform      (Shifts final image into viewport)
    pub bounds: na::Vector4<f32>,                       // Left/Right/Bottom/Top bounds     (Each ranging from 0 -> 2, basically the roots with the heights)
    pub color: na::Vector4<f32>,                        // Object Color
    pub pixel_box: glium::Rect,                         // Scissors the view so the image remains inside the pixel box
}

// Functionality for MVPetal data struct.
impl MVPetal {
    pub fn new(model: na::Matrix4<f32>, view: na::Matrix4<f32>, perspective: na::Matrix4<f32>, vp: na::Matrix4<f32>, bounds: na::base::Vector4<f32>, color: na::Vector4<f32>) -> MVPetal {
        MVPetal { 
            model: model, 
            view: view, 
            perspective: perspective, 
            vp: vp, 
            bounds: bounds, 
            color: color, 
            pixel_box: glium::Rect{
                left: 0, 
                bottom: 0, 
                width: 0, 
                height: 0
            } }
    }

    pub fn null() -> MVPetal {
        MVPetal { model: eye4(), view: eye4(), perspective: eye4(), vp: eye4(), bounds: full_range_vec(), color: na::base::Vector4::<f32>::new(0.0, 1.0, 0.0, 1.0), pixel_box: glium::Rect{left: 0,bottom: 0, width: 0, height: 0} }}

    pub fn update_view(&mut self, new_view: na::Matrix4<f32>) {
        self.view = new_view;
    }

    pub fn update_perspective(&mut self, new_perspective: na::Matrix4<f32>) {
        self. perspective = new_perspective;
    }

    pub fn update_viewport_pixel_bounds(&mut self, new_bounds: glium::Rect) {
        self.pixel_box = new_bounds;
    }

    pub fn null_from_view(view: na::Matrix4<f32>) -> MVPetal {
        MVPetal { 
            model: eye4(), 
            view: view, 
            perspective: eye4(), 
            vp:eye4(), 
            bounds: full_range_vec(), 
            color: na::base::Vector4::<f32>::new(0.0, 0.0, 1.0, 1.0), 
            pixel_box: glium::Rect{
                left: 0,
                bottom: 0, 
                width: 0, 
                height: 0}
         }
    }
}

// ################################################################################################

/* UTILITY FUNCTIONS */

// 4x1 vector for green color
pub fn green_vec() -> na::base::Vector4<f32> {
    na::base::Vector4::<f32>::new(0.0, 1.0, 0.0, 1.0)
}

// Cyan
pub fn cyan_vec() -> na::base::Vector4<f32> {
    na::base::Vector4::<f32>::new(0.0, 100.0/255.0, 100.0/255.0, 0.0)
}

// Red
pub fn red_vec() -> na::base::Vector4<f32> {
    na::base::Vector4::<f32>::new(1.0, 0.0, 0.0, 0.0)
}

// Blue
pub fn blue_vec() -> na::base::Vector4<f32> {
    na::base::Vector4::<f32>::new(0.0, 0.0, 1.0, 0.0)
}

// White
pub fn white_vec() -> na::base::Vector4<f32> {
    na::base::Vector4::<f32>::new(1.0, 1.0, 1.0, 1.0)
}

// 4x4 identity matrix
pub fn eye4() -> na::base::Matrix4<f32> {
    na::base::Matrix4::identity()
}

pub fn null3() -> na::base::Vector3<f64> {
    na::base::Vector3::new(0.0, 0.0, 0.0)
}

// Data type conversion function to make it usable for OpenGL
pub fn uniformifyMat4 (target: na::base::Matrix4<f32>) -> [[f32; 4]; 4] {
    *target.as_ref()
}

// Data type conversion function to make it usable for OpenGL
pub fn uniformifyVec4 (target: na::base::Vector4<f32>) -> [f32; 4] {
    *target.as_ref()
}

// translation matrix for XY plane
pub fn xy_translation(x: f32, y: f32) -> na::base::Matrix4<f32> {
    na::Matrix4::new_translation(
        &na::base::Vector3::<f32>::new(x, y, 0.0)
    )
}

// bounds for full range of screen
pub fn full_range() -> [f32; 4] {
    [-1.0, 1.0, -1.0, 1.0]
}

// bounds for full range in vec4 format
pub fn full_range_vec() -> na::base::Vector4<f32> {
    na::base::Vector4::new(
        -1.0,
        1.0, 
        -1.0, 
        1.0
    )
}

pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> na::base::Vector4<f32> {
    na::base::Vector4::new(
        r,
        g, 
        b, 
        a
    )
}

pub enum DrawType {
    Full,
    RotationOnly,
    PositionOnly,
    NoDraw,
}

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