/*  TO DO:
        - Implement drawtype into MVPetal struct
            - Determine appropriate drawtypes
                - Full
                - PositionOnly
                - RotationOnly
                - InheritPosition
                - InheritPositionAndRotation
                - InheritCamera
                - etc
            - create match cases for different passed draw types
                - e.g. RotationOnly should ignore position when 
*/

#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
#![allow(redundant_semicolons)]
#![allow(unused_assignments)]
#![allow(unreachable_patterns)]

// Imports
extern crate nalgebra as na;                                                // Linear Algebra 
extern crate glium;                                                         // OpenGL Bindings
use glium::{glutin::{self, window, event::{ElementState, VirtualKeyCode, ModifiersState}}, Surface};                               // OpenGL imports
use std::{rc::Rc, time::Instant, cell::RefCell, thread::scope, vec};        // Multithreading standard library items
mod scenes_and_entities;
extern crate tobj;                                                          // .obj file loader
extern crate rand;                                                          // Random number generator
extern crate pretty_env_logger;                                             // Logger
#[macro_use] extern crate log;                                              // Logging crate
mod dc;                                                                     // DATACOM interface module
mod isoviewer;                                                              // Isometric viewer struct 
mod wf;                                                                     // Wireframe struct
mod plt;                                                                    // Plotter
use crate::{dc::{Draw, cyan_vec, null_content, Draw2, green_vec, red_vec}, gp::Sim};                                 // DATACOM item imports for functions
use std::{thread, time::Duration, sync::{mpsc, Arc, Mutex, RwLock}};        // Multithreading lib imports
mod gp;                                                                     // 6DOF simulation
mod scene_composer;

fn main() {

    // Initialization procedures
    std::env::set_var("RUST_LOG", "trace");                                 // Initialize logger
    pretty_env_logger::init();
    info!("Starting Program");

    // PI constants
    let pi32 = std::f32::consts::PI;
    let pi64 = std::f64::consts::PI;

    // Matrix multiplication test
    // multiply_check();

    // Basic Blizzard Object
    let blizz_sim_obj = Arc::new(
        RwLock::new(
            create_blizzard()
        )
    );

    // Initialize glium items
    let event_loop = glutin::event_loop::EventLoop::new();                  // Create Event Loop
    let gui = dc::GuiContainer::init_opengl(&event_loop);                   // Initialize OpenGL interface

    // // Unit vectors for display
    // let i = Arc::new(RwLock::new(plt::Curve::unit_i(5.0)));                 // i hat - x direction unit vector 
    // let j = Arc::new(RwLock::new(plt::Curve::unit_j(5.0)));                 // j hat - y direction unit vector
    // let k = Arc::new(RwLock::new(plt::Curve::unit_k(5.0)));                 // k hat - z direction unit vector

    // // Plotter test
    // let tracer = plt::Curve::new(
    //     vec![],
    //     dc::rgba(
    //         0.0/255.0,
    //         100.0/255.0,
    //         100.0/255.0,
    //         0.0));
    // let tracer_ref = Arc::new(RwLock::new(tracer));

    // // 6DOF simulation test
    // let sim = Arc::new(
    //     gp::SIXDOF::new(
    //         vec![
    //             blizz_sim_obj.clone(),
    //         ]
    //     )
    // );

    // // Isometric viewer with unit vectors, 
    // let iso_test = Arc::new(
    //     isoviewer::IsoViewer::new(
    //         vec![
    //             i.clone(),
    //             j.clone(),
    //             k.clone(),
    //             blizz_sim_obj.clone(),
    //         ]
    //     )
    // );

    // // Isometric plotter test
    // let iso_camera_plotter = Arc::new(
    //     isoviewer::IsoViewer::new(
    //         vec![
    //             i.clone(),
    //             j.clone(),
    //             k.clone(),
    //             tracer_ref.clone()
    //         ]
    //     )
    // );

    // // Scope display test
    // let mut scope_test = plt::Scope::new(
    //     vec![
    //         tracer_ref.clone(),
    //         i.clone(),
    //         j.clone(),
    //         k.clone(),
    //     ]
    // );

    // // Setting range on the scope
    // scope_test.set_xrange(&[-10.0, 10.0]);
    // scope_test.set_yrange(&[-10.0, 10.0]);
    // let scope_test = Arc::new(scope_test);

    // // Create Viewports
    // let mut viewport_list = vec![
    //     // Main Viewport
    //     dc::Viewport::new_with_camera(
    //         na::Point2::new(-0.95, 0.9),            // Root Position
    //         1.85,                                   // Height
    //         1.0,                                    // Width
    //         sim.clone(),                            // Content
    //         na::Matrix4::look_at_rh(                // Camera Position
    //             &na::Point3::new(-5.0, 5.0, 5.0),   
    //             &na::Point3::new(0.0, 0.0, 0.0), 
    //             &na::Vector3::new(0.0, 0.0, 1.0)
    //         ),
    //     ),
    //     // 2D Scope item
    //     dc::Viewport::new_with_camera(
    //         na::Point2::new(0.05, 0.9),             // Root Position
    //         1.0,                                    // Height
    //         0.85,                                   // Width
    //         scope_test.clone(),                     // Content
    //         na::Matrix4::look_at_rh(                // Camera Position
    //             &na::Point3::new(0.0, 0.0, 1.0), 
    //             &na::Point3::new(0.0, 0.0, 0.0), 
    //             &na::Vector3::new(0.0, 1.0, 0.0)
    //         ),
    //     ),
    //     // 3D Trace plotter
    //     dc::Viewport::new_with_camera(
    //         na::Point2::new(0.05, -0.105), 
    //         0.85, 
    //         0.85, 
    //         iso_camera_plotter.clone(),
    //         na::Matrix4::look_at_rh(
    //             &na::Point3::new(20.0, 20.0, 20.0), 
    //             &na::Point3::new(0.0, 0.0, 0.0), 
    //             &na::Vector3::new(0.0, 0.0, 1.0)
    //         ),
    //     ),
    // ];

    // Null Content
    let mut null_test = Arc::new(
        null_content::new()
    );
    
    // let mut test_scene = scenes_and_entities::Scene::new();
    // let mut test_entity = scenes_and_entities::Entity::new();
    // let test_wireframe = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/blizzard.obj", green_vec());
    // let test_model = scenes_and_entities::ModelComponent::new(test_wireframe);
    // let test_wireframe_2 = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", red_vec());
    // let test_model_2 = scenes_and_entities::ModelComponent::new(test_wireframe_2);
    // test_entity.add_model(test_model);
    // test_entity.add_model(test_model_2);
    // test_scene.add_entity(test_entity);

    let test_scene = scene_composer::compose_scene_1();

    let test_scene_ref = Arc::new(test_scene);


    // Viewport Refactor Test

    let mut viewport_refactor = vec![
        dc::Twoport::new_with_camera(
            na::Point2::new(-0.98, 0.98), 
            0.98*2.0, 
            0.6*2.0, 
            test_scene_ref.clone(), 
            na::Matrix4::look_at_rh(                // Camera Position
                &na::Point3::new(-5.0, 5.0, 5.0),   
                &na::Point3::new(0.0, 0.0, 0.0), 
                &na::Vector3::new(0.0, 0.0, 1.0)
            )
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.22, 0.98), 
            0.4*2.0, 
            0.35*2.0, 
            null_test.clone(), 
            na::Matrix4::look_at_rh(                // Camera Position
                &na::Point3::new(0.0, 0.0, 1.0), 
                &na::Point3::new(0.0, 0.0, 0.0), 
                &na::Vector3::new(0.0, 1.0, 0.0)
            )
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.22, 0.98-0.8), 
            2.0*(0.98-0.4), 
            0.35*2.0, 
            null_test.clone(), 
            na::Matrix4::look_at_rh(                // Camera Position
                &na::Point3::new(0.0, 0.0, 1.0), 
                &na::Point3::new(0.0, 0.0, 0.0), 
                &na::Vector3::new(0.0, 1.0, 0.0)
            )
        ),
    ];

    // Framerate and clock items
    let frame_time_nanos = 16_666_667;
    let start_time = std::time::SystemTime::now();
    let mut t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);

    // Multithreading TRx
    let (tx_gui, rx_gui) = mpsc::sync_channel(1);

    // Thread for calculations
    thread::spawn(move || {
        loop {
            // Clock update
            t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);
            tx_gui.send(t).unwrap();

            // Log new position
            let new_pos = na::Point3::new((-5.0*&t.sin()) as f64, (5.0*&t.cos()) as f64, (5.0+2.0*(5.0*&t).sin()) as f64);

            // Write to tracer
            // {
            //     let mut w = tracer_ref.write().unwrap();
            //     (*w).add_point(&new_pos);
            //     // println!("{}", w.positions.len());
            // }
        }
    });

    // Draw thread
    (event_loop.run(move |event, _, control_flow| {
        let next_frame_time = std::time::Instant::now() + std::time::Duration::from_nanos(frame_time_nanos);
        let t = rx_gui.recv().unwrap();
        // let t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);

        // Event Handling
        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Left),
                        ..
                    },
                    ..
                } => println!("LEFT"),

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Right),
                        ..
                    },
                    ..
                } => println!("RIGHT"),

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Up),
                        ..
                    },
                    ..
                } => println!("UP"),

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Down),
                        ..
                    },
                    ..
                } => println!("DOWN"),

                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }

        // match event {
        //     glutin::event::Event::WindowEvent { event, .. } => match event {
        //         glutin::event::WindowEvent::CloseRequested => {
        //             *control_flow = glutin::event_loop::ControlFlow::Exit;
        //             return;
        //         },
        //         _ => return,
        //     },
        //     glutin::event::Event::NewEvents(cause) => match cause {
        //         glutin::event::StartCause::ResumeTimeReached { .. } => (),
        //         glutin::event::StartCause::Init => (),
        //         _ => return,
        //     },
        //     _ => return,
        // }

        

        

        let mut target = gui.display.draw();


        let new_pos = na::Point3::new(-5.0*&t.sin(), 5.0*&t.cos(), 5.0+2.0*(5.0*&t).sin());
        {
            let mut w = blizz_sim_obj.write().unwrap();
            // (*w).advance_state();
            (*w).update(t as f64)
        }

        

        // viewport_list[0].update_camera(
        //     na::Matrix4::look_at_rh(
        //         &new_pos,
        //         &na::Point3::new(0.0, 0.0, 0.0), 
        //         &na::Vector3::new(0.0, 0.0, 1.0)
        //     )
        // );

        // viewport_list[1].update_camera(na::Matrix4::look_at_rh(
        //     &na::Point3::new(5.0*&t.sin(), 5.0*&t.cos(), 5.0), 
        //     &na::Point3::new(0.0, 0.0, 0.0), 
        //     &na::Vector3::new(0.0, 0.0, 1.0)
        // ));
        
        // for i in &mut viewport_list {
        //     i.update_all_graphical_elements(&target);
        // }

        target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
        // for i in &viewport_list {
        //     i.draw(&gui, &i.mvp, &mut target);
        // }

        for i in &mut viewport_refactor {
            i.update_all_graphical_elements(&target)
        }

        for i in &viewport_refactor {
            i.draw(&gui, &i.context, &mut target)
        }

        target.finish().unwrap();

        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

    }));

}

fn create_six_dof() -> gp::StateSpace<f64, 12, 8, 7> {
    let A = na::base::SMatrix::<f64, 12, 12>::zeros();
    let B = na::base::SMatrix::<f64, 12, 8>::zeros();
    let C = na::base::SMatrix::<f64, 7, 12>::from_row_slice(&[
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,         // x position
        0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,         // y position
        0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,         // z position
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,         // Q 1 
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,         // Q 2
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,         // Q 3
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,         // Q 4
    ]);

    let D = na::base::SMatrix::<f64, 7, 8>::zeros();
    let x = na::base::SVector::<f64, 12>::zeros();
    let u = na::base::SVector::<f64, 8>::zeros();
    let y = na::base::SVector::<f64, 7>::zeros();
    gp::StateSpace::new(A, B, C, D, x, u, y)
}

fn create_blizzard() -> gp::SimObj<f64, 12, 8, 7> {

    // Blizzard Model
    let blizzard = Arc::new(RwLock::new(wf::Wireframe::load_wireframe_from_obj("data/blizzard.obj", dc::cyan_vec())));

    // Rotor wireframe model
    let prop_red =Arc::new(
        wf::Wireframe::load_wireframe_from_obj("data/prop.obj", dc::red_vec())
    );

    let prop_blue =Arc::new(
        wf::Wireframe::load_wireframe_from_obj("data/prop.obj", dc::blue_vec())
    );

    let sixdof = create_six_dof();

    gp::SimObj::<f64,12,8,7>::new(
        blizzard.clone(),
        vec![
            Arc::new(   // Front Right Bottom
                RwLock::new(
                    gp::Rotor::new(
                        prop_red.clone(),
                        na::base::Vector3::new(-0.72, 2.928, 1.041-0.15),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                    )
                )
            ),
            Arc::new(   // Front Right Top
                RwLock::new(
                    gp::Rotor::new(
                        prop_blue.clone(),
                        na::base::Vector3::new(-0.72, 2.928, 1.041+0.15),
                        na::base::Vector3::new(0.0, 0.0, -1.0),
                        na::base::Vector3::new(0.0, 0.0, -1.0),
                    )
                )
            ),
            Arc::new(   // Front Left Bottom
                RwLock::new(
                    gp::Rotor::new(
                        prop_blue.clone(),
                        na::base::Vector3::new(-0.72, -2.928, 1.041-0.15),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                        na::base::Vector3::new(0.0, 0.0, -1.0),
                    )
                )
            ),
            Arc::new(   // Front Left Top
                RwLock::new(  
                    gp::Rotor::new(
                        prop_red.clone(),
                        na::base::Vector3::new(-0.72, -2.928, 1.041+0.15),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                    )
                )
            ),
            Arc::new(   // Back Right Bottom
                RwLock::new(
                    gp::Rotor::new(
                        prop_blue.clone(),
                        na::base::Vector3::new(4.220, 2.928, 1.041-0.15),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                        na::base::Vector3::new(0.0, 0.0, -1.0)
                    )
                )
            ),
            Arc::new(   // Back Right Top
                RwLock::new(
                    gp::Rotor::new(
                        prop_red.clone(),
                        na::base::Vector3::new(4.220, 2.928, 1.041+0.15),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                    )
                )
            ),
            Arc::new(   // Back Left Bottom
                RwLock::new(
                    gp::Rotor::new(
                        prop_red.clone(),
                        na::base::Vector3::new(4.220, -2.928, 1.041-0.15),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                        na::base::Vector3::new(0.0, 0.0, 1.0)
                    )
                )
            ),
            Arc::new(   // Back Left Top
                RwLock::new(
                    gp::Rotor::new(
                        prop_blue.clone(),
                        na::base::Vector3::new(4.220, -2.928, 1.041+0.15),
                        na::base::Vector3::new(0.0, 0.0, 1.0),
                        na::base::Vector3::new(0.0, 0.0, -1.0)
                    )
                )
            ),
            ],
        dc::MVPetal::null(),
        sixdof
    )
}

#[cfg(test)]
mod tests {
    use crate::{create_blizzard, gp::Sim};

    
    #[test]
    fn multiply_check() {
        let B = na::base::SMatrix::<f64, 12, 8>::zeros();
        let u = na::base::SVector::<f64, 8>::zeros();

        let xdot = B*u;

        assert_eq!(xdot, na::base::SVector::<f64, 12>::zeros());
    }

    #[test]
    fn observation_checl() {
        let blizz = create_blizzard();
        // let y_init = blizz.observe_full_state(na::base::SVector::<f64, 8>::zeros());
        // assert_eq!(y_init, (xdot, na::base::SVector::<f64, 7>::zeros());
    }
}