/*  TO DO:
    // indicates completed.
    -Camera
        - Implement various camera behaviors
            - Tracking
                - Camera with a static position locks on to a moving object
            - Following 
                - Camera is static relative to moving current_frame
    - Behaviors
        // - Create command that maps to behavior
        // - Create command that modifies behaviors in situ
        - Create command that deletes behaviors in situ
        // - Allow behaviors to be created from json file
        // - Create function to iterate over all behaviors in an entity
    - 2D Display
        - Basics
            - Create 2D Viewport
            - Create 2D render context
            - Create 2D primitives (Vertex, Normals)
            - Create 2D shape primitives
                - Ellipse
                - Circle
                - Line
                - Square
        - More Advanced
            - Flatten 3D scene and display as 2D
            - 
    - JSON parsing and loading
        // - Make models loadable from JSON
        // - Make entities loadable from JSON
        - Make wireframe loadable from JSON
        // - Make behaviors loadable from JSON
        // - Make scenes loadable from JSON
        // - Entities commandable from JSON
        // - All entities in scene can be commanded over JSON
*/

#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
#![allow(redundant_semicolons)]
#![allow(unused_assignments)]
#![allow(unreachable_patterns)]
#![allow(unused_mut)]
// Imports
extern crate nalgebra as na;                                                // Linear Algebra 
extern crate glium;                                                         // OpenGL Bindings
use glium::{glutin::{self, 
    window, 
    event::{ElementState, VirtualKeyCode, ModifiersState}},
     Surface};                                                              // OpenGL imports
use core::time;
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
use crate::dc::{Draw, cyan_vec, null_content, 
    Draw2, green_vec, red_vec};                                             // DATACOM item imports for functions
use std::{thread, time::Duration, sync::{mpsc, Arc, Mutex, RwLock}};        // Multithreading lib imports
mod scene_composer;

fn main() {

    // Initialization procedures
    std::env::set_var("RUST_LOG", "trace");                                 // Initialize logger
    pretty_env_logger::init();
    info!("Starting Program");

    // let test_scene = scene_composer::compose_scene_3();

    let test_scene = scenes_and_entities::Scene::load_from_json_file("data/scene_loading/test_scene.json");

    start_program(test_scene);
}

fn start_program(scene: scenes_and_entities::Scene) {

    // PI constants
    let pi32 = std::f32::consts::PI;
    let pi64 = std::f64::consts::PI;

    // Initialize glium items
    let event_loop = glutin::event_loop::EventLoop::new();                  // Create Event Loop
    let gui = dc::GuiContainer::init_opengl(&event_loop);                   // Initialize OpenGL interface

    let scene_ref = Arc::new(RwLock::new(scene));
    let scene_ref_2 = scene_ref.clone();


    // Viewport Refactor Test

    let mut viewport_refactor = vec![
        dc::Twoport::new_with_camera(
            na::Point2::new(-0.98, 0.98), 
            0.98*2.0, 
            0.6*2.0, 
            scene_ref.clone(),
            na::Point3::new(-7.0, 3.0, 5.0),
            na::Point3::new(0.0, 0.0, 0.0)
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.22, 0.98), 
            0.4*2.0, 
            0.35*2.0, 
            scene_ref.clone(), 
            na::Point3::new(10.0, 0.0, 0.0),
            na::Point3::new(0.0, 0.0, 0.0)
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.22, 0.98-0.8), 
            2.0*(0.98-0.4), 
            0.35*2.0, 
            scene_ref.clone(), 
            na::Point3::new(0.0, 10.0, 1.0), 
            na::Point3::new(0.0, 0.0, 0.0),
        ),

        // dc::Twoport::new_with_camera(
        //     na::Point2::new(-1.0, 1.0), 
        //     1.0*2.0, 
        //     1.0*2.0, 
        //     scene_ref.clone(),
        //     na::Point3::new(-7.0, 3.0, 5.0),
        //     na::Point3::new(0.0, 0.0, 0.0)
        // ),
    ];

    // Framerate and clock items
    let frame_time_nanos = 16_666_667;
    // let frame_time_nanos = 33_333_333;
    let start_time = std::time::SystemTime::now();
    let mut t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);

    // TEST CODE
    let str = std::fs::read_to_string("data/test_commands/test_command.json").unwrap();
    scene_ref_2.write().unwrap().cmd_msg_str(&str);
    // END TEST

    // Multithreading TRx
    let (tx_gui, rx_gui) = mpsc::sync_channel(1);
    // Thread for calculations
    let calculation_thread = thread::Builder::new().name("calculation thread".to_string()).spawn(move || {
        loop {
            // Clock update
            t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);
            tx_gui.send(t).unwrap();

            // TEST CODE
            let test_command_data = format!("
                {{
                    \"targetEntityID\": 0,
                    \"commandType\": \"ComponentChangeColor\",
                    \"data\": [0.0,{},{},{},1.0]
                }}
            ", 
            t.sin().abs(),
            t.cos().abs(),
            t.tan().abs()
        );
            // let test_command_json: serde_json::Value = serde_json::from_str(test_command_data.as_str()).unwrap();
            scene_ref_2.write().unwrap().cmd_msg_str(&test_command_data.as_str());
            // END TEST CODE

            // Scene update
            scene_ref_2.write().unwrap().update();
        }
    });

    // Draw thread
    (event_loop.run(move |event, _, control_flow| {
        let next_frame_time = std::time::Instant::now() + std::time::Duration::from_nanos(frame_time_nanos);
        let t = rx_gui.recv().unwrap();
        // let t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);

        // Event Handling (Key presses, mouse movement)
        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                },

                // Do this when the mouse moves 
                glutin::event::WindowEvent::CursorMoved { 
                    position,
                    ..
                 } => {
                    for viewport in &mut viewport_refactor {
                        if viewport.in_viewport(&gui, &(position.x as u32), &(position.y as u32)) {
                            viewport.set_active();
                        }
                        else {
                            viewport.set_inactive();
                        }
                    }
                 },


                // Zoom when mouse wheel moved
                glutin::event::WindowEvent::MouseWheel { 
                delta,
                ..
                } => {
                for viewport in &mut viewport_refactor {
                    if viewport.is_active {
                        viewport.zoom(delta);
                        debug!("ZOOM")
                    }
                }
                
                },
                
                // Pan Left using left arrow key
                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Left),
                        ..
                    },
                    ..
                } => {
                    debug!("LEFT");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.move_camera(na::Vector3::<f64>::new(0.0, -1.0, 0.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                },

                // Pan Right using right arrow key
                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Right),
                        ..
                    },
                    ..
                } => {
                    debug!("RIGHT");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.move_camera(na::Vector3::<f64>::new(0.0, 1.0, 0.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                }
                    // viewport_refactor[0].move_camera(na::Vector3::<f64>::new(0.0, 1.0, 0.0));
                    

                // Pan Up using up arrow key
                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Up),
                        ..
                    },
                    ..
                } => {
                    debug!("UP");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.move_camera(na::Vector3::<f64>::new(0.0, 0.0, 1.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                },

                // Pan Down using down arrow key
                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Down),
                        ..
                    },
                    ..
                } => {
                    debug!("DOWN");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.move_camera(na::Vector3::<f64>::new(0.0, 0.0, -1.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::A),
                        ..
                    },
                    ..
                } => {
                    debug!("A");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.orbit(-5.0, 0.0, na::base::Vector3::new(0.0, 0.0, 1.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::D),
                        ..
                    },
                    ..
                } => {
                    debug!("D");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.orbit(5.0, 0.0,  na::base::Vector3::new(0.0, 0.0, 1.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::W),
                        ..
                    },
                    ..
                } => {
                    debug!("W");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.orbit(0.0, -5.0,  na::base::Vector3::new(0.0, 0.0, 1.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::S),
                        ..
                    },
                    ..
                } => {
                    debug!("S");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.orbit(0.0, 5.0,  na::base::Vector3::new(0.0, 0.0, 1.0));
                            debug!("{}", viewport.camera_position);
                        }
                    }
                },
                

                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }
        let mut current_frame = gui.display.draw();

        current_frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        
        let seconds_per_rotation: f64 = 5.0;

        viewport_refactor[0].change_camera_position(na::Point3::<f64>::new(
            7.0*(t/1.0).cos() as f64,
            7.0*(t/1.0).sin() as f64,
            0.0,
        ));
        

        for i in &mut viewport_refactor {
            i.update_all_graphical_elements(&current_frame)
        }

        for i in &viewport_refactor {
            i.draw(&gui, &i.context, &mut current_frame)
        }

        current_frame.finish().unwrap();

        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);


    }));
}

#[cfg(test)]
mod tests {
    use crate::{scene_composer, scenes_and_entities::{self, ModelComponent}};


    #[test]
    fn unit_quaternion() {
        let unit_quaternion: na::UnitQuaternion<f64> = na::UnitQuaternion::identity();
        info!("{}", unit_quaternion);
    }

    fn load_from_json(){
        scenes_and_entities::ModelComponent::load_from_json_file(&"data/object_loading.blizzard_initialize.json");
        
    }

    #[test]
    fn color_change() {
        let mut test_scene = scene_composer::test_scene();
        let color_cmd = scenes_and_entities::Command::new(
            scenes_and_entities::CommandType::ComponentChangeColor,
            vec![0.0, 1.0, 1.0, 1.0, 1.0]
        );
        assert_eq!(
            test_scene.get_entity(0).get_model(0).get_color(),
            na::Vector4::<f32>::new(0.0, 1.0, 0.0, 1.0),
            "Base color is green"
        );
        test_scene.get_entity(0).command(color_cmd);
        assert_eq!(
            test_scene.get_entity(0).get_model(0).get_color(),
            na::Vector4::<f32>::new(1.0, 1.0, 1.0, 1.0),
            "New color is white"
        );
    }

    #[test]
    fn position_change() {
        let mut test_scene = scene_composer::test_scene();
        let pos_cmd = scenes_and_entities::Command::new(
            scenes_and_entities::CommandType::EntityChangePosition,
            vec![1.0, 1.0, 1.0]
        );
        assert_eq!(
            test_scene.get_entity(0).get_position(),
            na::Point3::<f64>::origin(),
            "Initial Position is Origin"
        );
        test_scene.get_entity(0).command(pos_cmd);
        assert_eq!(
            test_scene.get_entity(0).get_position(),
            na::Point3::<f64>::new(1.0, 1.0, 1.0),
            "Position commanded successfully"
        );
    }

    #[test]
    fn change_command() {
        let mut test_scene = scene_composer::test_scene();
        let change_command = scenes_and_entities::Command::new(
            scenes_and_entities::CommandType::ModifyBehavior, 
            vec![0.0, ]
        );
    }


}