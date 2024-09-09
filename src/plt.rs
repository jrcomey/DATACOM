// use crate::dc;
// use crate::glium::Surface;
// use std::sync::{Arc, RwLock};

// // ################################################################################################

// pub struct Scope {
//     curves:     Vec<Arc<RwLock<Curve>>>,
//     x_min:      f64,
//     x_max:      f64,
//     y_min:      f64,
//     y_max:      f64,
// }

// impl Scope {
//     pub fn new(curves: Vec<Arc<RwLock<Curve>>>) -> Scope {
//         Scope {
//             curves,
//             x_min: -1.0, 
//             x_max: 1.0, 
//             y_min: -1.0, 
//             y_max: 1.0 
//         }
//     }

//     pub fn set_xrange(&mut self, xrange_new: &[f64; 2]) {
//         self.x_min = xrange_new[0];
//         self.x_max = xrange_new[1];
//     }

//     pub fn set_yrange(&mut self, yrange_new: &[f64; 2]) {
//         self.y_min = yrange_new[0];
//         self.y_max = yrange_new[1];
//     }
// }

// impl dc::Draw for Scope {
//     fn draw(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
//         // Create uniforms

//         // info!("Scope Draw Call");

//         // let uniforms = glium::uniform! {
//         //     model: dc::uniformifyMat4(mvp.model),
//         //     view: dc::uniformifyMat4(mvp.view),
//         //     perspective: dc::uniformifyMat4(dc::eye4()),        // No perspective on 2D Plots
//         //     color_obj: dc::uniformifyVec4(dc::cyan_vec()),      // Default cyan color. TODO: Make configurable
//         //     vp: dc::uniformifyMat4(mvp.vp),                     // Viewport positioning on screen
//         //     bounds: dc::uniformifyVec4(mvp.bounds),
//         // };

//         let xaxis = Curve::new(
//             vec![
//                 na::Point3::new(self.x_min, 0.0, 0.0),
//                 na::Point3::new(self.x_max, 0.0, 0.0)
//             ],
//             dc::green_vec()
//         );

//         let yaxis = Curve::new(
//             vec![
//                 na::Point3::new(0.0, self.y_min, 0.0),
//                 na::Point3::new(0.0, self.y_max, 0.0),
//             ],
//             dc::green_vec()
//         );

//         // let test_line = Arc::new(
//         //     RwLock::new(
//         //         Curve::new(
//         //             vec![
//         //                 na::Point3::new(-1.0, 1.0, 0.0),
//         //                 na::Point3::new(1.0, -1.0, 0.0)
//         //             ],
//         //             dc::red_vec()
//         //         )
//         //     )
//         // );

//         let model = na::Matrix4::new_nonuniform_scaling(
//             &na::Vector3::<f32>::new(
//                 (mvp.bounds[1]-mvp.bounds[0])/(self.x_max-self.x_min) as f32, 
//                 (mvp.bounds[3]-mvp.bounds[2])/(self.y_max-self.y_min) as f32, 
//                 0.0));
//         // let view = na::Matrix4::look_at_rh(
//         //     &na::Point3::new(1.0, 0.0, 0.0),
//         //     &na::Point3::new(0.0, 0.0, 0.0), 
//         //     &na::Vector3::new(0.0, 1.0, 0.0)
//         // );
//         let view = mvp.view;                                    // Inherit Camera View
//         let projection = na::base::Matrix4::new_orthographic(   // Orthographic projection
//             -1.0, 
//             1.0, 
//             -1.0, 
//             1.0, 
//             -10.0, 
//             10.0);
//         let vp = mvp.vp;                                        // Scale to Viewport position
//         let bounds = mvp.bounds;                                // Bounds of viewport
//         let color = dc::green_vec();                            // Line Color for axes
//         let pixel_box = mvp.pixel_box;                          // Cutoffs for viewport

//         let mvp_local = dc::MVPetal{
//             model,
//             view,
//             perspective: projection,
//             vp,
//             bounds,
//             color,
//             pixel_box,
//         };

//         // println!("{}", mvp.perspective);
//         // println!("{}", mvp_local.perspective);

//         xaxis.draw(&gui, &mvp_local, target);
//         yaxis.draw(&gui, &mvp_local, target);
//         for i in &self.curves {
//             i.read().unwrap().draw(&gui, &mvp_local, target);
//         }
//         // test_line.read().unwrap().draw(&gui, &mvp_local, target);
//     }

    
//     fn draw_absolute(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
//         ;
//     }
// }

// // ################################################################################################

// // pub struct Datalogger {
// //     data:               Curve,          // Data that is being stored
// //     time_data:          Vec<f64>,       // Time index of data being stored
// //     current_time_ms:    f64,            // Current time index
// //     max_time_kept:      f64,            // Maximum index of data to be kept (e.g. keep last 30 seconds). If 0,, keep all data.
// // }

// // impl Datalogger {
// //     pub fn get_data(&self) -> (Curve, Vec<f64>) {
// //         return (self.data, self.time_data)
// //     }
// // }

// // ################################################################################################

// // #[derive(Debug, Copy, Clone)]
// pub struct Curve {
//     pub positions:  Vec<dc::Vertex>,
//     indices:        Vec<u32>,
//     color:          na::base::Vector4<f32>
// }

// impl Curve {
//     pub fn new(point_vector: Vec<na::Point3<f64>>, color: na::base::Vector4<f32>) -> Curve {
//         let mut position_vector: Vec<dc::Vertex> = vec![];
//         let mut index_vector: Vec<u32> = vec![];
//         for (i, point) in point_vector.iter().enumerate() {
//             position_vector.push(dc::Vertex { position: [point[0], point[1], point[2]] });
//             index_vector.push(i as u32);
//         };
//         Curve { positions: position_vector, indices: index_vector, color: color }
//     }

//     pub fn add_point(&mut self, new_point: &na::Point3<f64>) {
//         self.positions.push(
//             dc::Vertex { position: [new_point[0], new_point[1], new_point[2]]}
//         );
//         self.indices.push(
//             self.indices.len() as u32
//         );
//     }

//     pub fn two_point_line(point_0: na::Point3<f64>, point_1: na::Point3<f64>, color: na::base::Vector4<f32>) -> Curve {
//         Curve::new(
//             vec![point_0, point_1],
//             color
//         )
//     }

//     pub fn unit_i(magnitude: f64) -> Curve {
//         Curve::two_point_line(
//             na::Point3::<f64>::new(0.0, 0.0, 0.0), na::Point3::<f64>::new(magnitude, 0.0, 0.0), dc::rgba(1.0, 0.0, 0.0, 0.0))
//     }

//     pub fn unit_j(magnitude: f64) -> Curve {
//         Curve::two_point_line(
//             na::Point3::<f64>::new(0.0, 0.0, 0.0), na::Point3::<f64>::new(0.0, magnitude, 0.0), dc::rgba(0.0, 1.0, 0.0, 0.0))
//     }

//     pub fn unit_k(magnitude: f64) -> Curve {
//         Curve::two_point_line(
//             na::Point3::<f64>::new(0.0, 0.0, 0.0), na::Point3::<f64>::new(0.0, 0.0, magnitude), dc::rgba(0.0, 0.0, 1.0, 0.0))
//     }
// }

// impl dc::Draw for Curve {

//     fn draw(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
//         // Create uniforms
//         let uniforms = glium::uniform! {
//             model: dc::uniformifyMat4(mvp.model),
//             view: dc::uniformifyMat4(mvp.view),
//             perspective: dc::uniformifyMat4(mvp.perspective),
//             color_obj: dc::uniformifyVec4(self.color),
//             vp: dc::uniformifyMat4(mvp.vp),
//             bounds: dc::uniformifyVec4(mvp.bounds),
//         };

//         // Buffer definitions
//         // let index_list: [u16; 4] = [0, 1, 2, 3];
//         let positions = glium::VertexBuffer::new(&gui.display, &self.positions).unwrap();
//         let indices = glium::IndexBuffer::new(&gui.display, glium::index::PrimitiveType::LineStrip, &self.indices).unwrap();

        

//         let params = glium::DrawParameters {
//             polygon_mode: glium::draw_parameters::PolygonMode::Line,
//             depth: glium::Depth {
//                 test: glium::draw_parameters::DepthTest::IfLess,
//                 write: true,
//                 ..Default::default()
//             },
//             // line_width: std::option::Option::Some(1E-0),
//             scissor: std::option::Option::Some(mvp.pixel_box),
//             backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
//             // polygon_offset: poly_off,
//             ..Default::default()
//         };

//         // Call glium draw function
//         target.draw(
//             &positions, 
//             &indices, 
//             &gui.program, 
//             &uniforms, 
//             &params)
//                 .unwrap();
//     }

//     fn draw_absolute(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
//         error!("Not implemented!");
//     }
// }