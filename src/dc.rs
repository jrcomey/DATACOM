extern crate glium;
use std::{rc::Rc, sync::Arc, time::Instant};
use glium::{glutin::{self, window}, Surface};

// ##################### 

/* VIEWPORT TAKE 2 */

pub struct Twoport {
    pub root: na::Point2<f64>,                          // Upper left window coordinate
    pub height: f64,                                    // Height of window from top -> down side   (RANGE 0 -> 2)
    pub width: f64,                                     // Width of window from left -> right side  (RANGE 0 -> 2)
    pub content: Arc<dyn Draw2>,                         // Pointer to content to be drawn           
    pub context: RenderContext,                         // Local Viewport Render Context
}

impl Twoport {

    pub fn new_with_camera(root: na::Point2<f64>, height: f64, width: f64, content: Arc<dyn Draw2>, view: na::Matrix4<f32>) -> Twoport {
        Twoport {
            root: root,
            height: height,
            width: width,
            content: content,
            context: RenderContext::new(
                view, 
                na::Matrix4::new_perspective((3.0/2.0) as f32, 3.141592 / 3.0, 0.1, 1024.0),
                xy_translation((root[0]+width/2.0) as f32, (root[1]-height/2.0) as f32),
                glium::Rect{
                    left: 0, 
                    bottom: 0, 
                    width: 0, 
                    height: 0
                }
            )
        }
    }

    pub fn update_all_graphical_elements(&mut self, target: &glium::Frame) {
        let (width, height) = target.get_dimensions();

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
}

impl Draw2 for Twoport {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame) {
        // println!("Drawing window...");

        // Create uniforms
        let uniforms = glium::uniform! {
            model: uniformifyMat4(eye4()),              // Identity matrix for M, not moving anywhere
            view: uniformifyMat4(eye4()),               // Identity matrix for V, should be viewed dead on
            perspective: uniformifyMat4(eye4()),        // Identity matrix for P, should not have perspective
            color_obj: uniformifyVec4(cyan_vec()),       // Use set color
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
        self.content.draw(&gui, &self.context, target); // Draw dependent content
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

pub trait Draw2 {
    fn draw(&self, gui: &GuiContainer, context: &RenderContext, target: &mut glium::Frame);
}












































































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
    pub root: na::Point2<f64>,
    pub height: f64,
    pub width: f64,
    pub content: Arc<dyn Draw>,
    pub mvp: MVPetal,
}

/* VIEWPORT SPECIFIC FUNCTIONALITY */
impl Viewport {

    // Standard viewport creation. Defines position, dimensions, and a link to content.
    pub fn new(root: na::Point2<f64>, h: f64, w: f64, content: Arc<dyn Draw>) -> Viewport {
        Viewport {
            root: root,                                 // Upper left root coordinate
            height: h,                                  // height of window from top->down side
            width: w,                                   // width of window from left->right side
            content: content,                           // Pointer to dependent content (something drawable!)
            mvp: MVPetal::null(),                       // Storage matrix with viewport camera
        }
    }
    
    // Viewport creation. Defines position, dimensions, and a link to content. Sets mvp struct as well.
    pub fn new_with_mvp(root: na::Point2<f64>, h: f64, w: f64, content: Arc<dyn Draw>, mvp: MVPetal) -> Viewport {
        Viewport {
            root: root,                                 // Upper left root coordinate
            height: h,                                  // height of window from top->down side
            width: w,                                   // width of window from left->right side
            content: content,                           // Pointer to dependent content (something drawable!)
            mvp: mvp,                                   // Storage matrix with viewport camera
        }
    }

    // Standard viewport creation. Defines position, dimensions, and a link to content.
    pub fn new_with_camera(root: na::Point2<f64>, h: f64, w: f64, content: Arc<dyn Draw>, view: na::Matrix4<f32>) -> Viewport {
        Viewport {
            root: root,                                 // Upper left root coordinate
            height: h,                                  // height of window from top->down side
            width: w,                                   // width of window from left->right side
            content: content,                           // Pointer to dependent content (something drawable!)
            mvp: MVPetal::new(
                eye4(),                                                                         // Model
                view,                                                                           // View
                na::Matrix4::new_perspective((3.0/2.0) as f32, 3.141592 / 3.0, 0.1, 1024.0),    // Perspective
                xy_translation((root[0]+w/2.0) as f32, (root[1]-h/2.0) as f32),                 // Viewport Shift
                na::base::Vector4::<f32>::new(                                                  // Bounds
                    (root[0]) as f32,   // Left X
                    (root[0]+w) as f32, // Right X
                    (root[1]-h) as f32, // Bottom Y 
                    (root[1]) as f32    // Top Y
                ),
                green_vec()                                                                     // Color
            ),         // Storage matrix with viewport camera
        }
    }

    // Test viewport creation function. Creates a static viewport with fixed coordinates and a null content link.
    pub fn null() -> Viewport {
        Viewport {
            root: na::Point2::new(0.0, 0.0),            // Upper left root coordinate
            height: 0.25,                               // height of window from top->down side
            width: 0.25,                                // width of window from left->right side
            content: Arc::new(null_content::new()),      // Pointer to dependent content (something drawable!)
            mvp: MVPetal::null(),                       // Storage matrix with viewport camera
        }
    }

    pub fn update_all_graphical_elements(&mut self, target: &glium::Frame) {
        let (width, height) = target.get_dimensions();
        let boxxy = glium::Rect{
            left: (((self.mvp.bounds[0]+1.0)/2.0) * width as f32) as u32,
            bottom: (((self.mvp.bounds[2]+1.0)/2.0) * height as f32) as u32,
            width: ((self.mvp.bounds[1]-self.mvp.bounds[0])/2.0*width as f32) as u32,
            height: ((self.mvp.bounds[3]-self.mvp.bounds[2])/2.0*height as f32) as u32};
        self.update_perspective(na::base::Matrix4::new_perspective(width as f32/height as f32, 3.141592 / 3.0, 0.1 , 1024.0));
        // self.update_perspective(na::base::Matrix4::new_orthographic((((self.mvp.bounds[0]+1.0)/2.0) * width as f32), right, bottom, top, znear, zfar))
        self.update_cutoffs(boxxy);
    }

    pub fn update_perspective(&mut self, new_perspective: na::Matrix4<f32>) {
        self.mvp.update_perspective(new_perspective);
    }

    pub fn update_cutoffs(&mut self, new_pixel_bounds: glium::Rect) {
        self.mvp.update_viewport_pixel_bounds(new_pixel_bounds);
    }

    pub fn update_camera(&mut self, new_camera: na::Matrix4<f32>) {
        self.mvp.update_view(new_camera);
    }
}

/* VIEWPORT DRAW TRAIT */
impl Draw for Viewport {
    fn draw(&self, gui: &GuiContainer, mvp: &MVPetal, target: &mut glium::Frame) {
        // println!("Drawing window...");

        // Create uniforms
        let uniforms = glium::uniform! {
            model: uniformifyMat4(eye4()),              // Identity matrix for M, not moving anywhere
            view: uniformifyMat4(eye4()),               // Identity matrix for V, should be viewed dead on
            perspective: uniformifyMat4(eye4()),        // Identity matrix for P, should not have perspective
            color_obj: uniformifyVec4(mvp.color),       // Use set color
            vp: uniformifyMat4(eye4()),                 // Identity matrix for post processing move. Viewport is stationary relative to itself!
            bounds: uniformifyVec4(full_range_vec()),   // Viewports should be able to take up the whole screen
        };

        // Define Positions, if necessary
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
        self.content.draw(&gui, &self.mvp, target); // Draw dependent content

    }

    fn draw_absolute(&self, gui: &GuiContainer, mvp: &MVPetal, target: &mut glium::Frame) {
        error!("Not implemented!");
    }
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