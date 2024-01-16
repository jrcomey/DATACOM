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

    // Null Content
    let mut null_test = Arc::new(
        null_content::new()
    );

    let test_scene = scene_composer::compose_scene_1();

    let test_scene_ref = Arc::new(test_scene);


    // Viewport Refactor Test

    let mut viewport_refactor = vec![
        dc::Twoport::new_with_camera(
            na::Point2::new(-0.98, 0.98), 
            0.98*2.0, 
            0.6*2.0, 
            test_scene_ref.clone(),
            na::Point3::new(-7.0, 3.0, 5.0),
            na::Point3::new(0.0, 0.0, 0.0)
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.22, 0.98), 
            0.4*2.0, 
            0.35*2.0, 
            test_scene_ref.clone(), 
            na::Point3::new(10.0, 0.0, 0.0),
            na::Point3::new(0.0, 0.0, 0.0)
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.22, 0.98-0.8), 
            2.0*(0.98-0.4), 
            0.35*2.0, 
            test_scene_ref.clone(), 
            na::Point3::new(0.0, 10.0, 1.0), 
            na::Point3::new(0.0, 0.0, 0.0),
            
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
        let mut target = gui.display.draw();

        target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

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

#[cfg(test)]
mod tests {
}