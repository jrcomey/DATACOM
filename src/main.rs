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
        // - Make behaviors loadable from JSON
        // - Make scenes loadable from JSON
        // - Entities commandable from JSON
        // - All entities in scene can be commanded over JSON
    - Networking
        // - Commands sendable over TCP connectio/n
        // - Commands are receivable over TCP connection
        // - Multiple commands can be sent in the same json
        // - Load Scene from network
    - Scene Playback
        - Load playback scene from file
            - Play scene in real time
            - Play scene at half, double speed
            - Play scene frame-by-frame, with ability to advance frame
    - Text Rendering
        -
*/

// #![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
// #![allow(redundant_semicolons)]
#![allow(unused_assignments)]
// #![allow(unreachable_patterns)]
#![allow(unused_mut)]
// Imports
extern crate nalgebra as na;                                                // Linear Algebra 
extern crate glium;                                                         use dc::{RenderContext, Text};
// OpenGL Bindings
use glium::{buffer, debug, texture::RawImage2d, Texture2d, glutin::{self, event::{ElementState, ModifiersState, VirtualKeyCode}, window}, Surface};
use num_traits::ops::bytes;
use scene_composer::test_scene;                                             // OpenGL imports
// use glium:;
use glium::debug::DebugCallbackBehavior;
use rusttype;
use image;
use text::{create_texture_atlas, TextDisplay};
use core::time;
use std::{cell::RefCell, collections::HashMap, io::Write, rc::Rc, thread::scope, time::Instant, vec};        // Multithreading standard library items
mod scenes_and_entities;
extern crate tobj;                                                          // .obj file loader
extern crate rand;                                                          // Random number generator
extern crate pretty_env_logger;                                             // Logger
#[macro_use] extern crate log;                                              // Logging crate
mod dc;                                                                     // DATACOM interface module
mod plt;                                                                    // Plotter
use crate::dc::{cyan_vec, null_content, 
    Draw2, CameraControl, green_vec, red_vec};                                             // DATACOM item imports for functions
use std::{thread, time::Duration, sync::{mpsc, Arc, Mutex, RwLock}};        // Multithreading lib imports
mod scene_composer;
use std::net::{SocketAddr, TcpListener, TcpStream};
mod com;
mod text;

fn main() {

    // Initialization procedures
    std::env::set_var("RUST_LOG", "trace");                                 // Initialize logger
    pretty_env_logger::init();
    info!("Program Start!");

    // let test_scene = scene_composer::compose_scene_3();
    // let recv_thread = thread::spawn(move|| {
    //     // let test_scene = load_scene_from_network("localhost:8080".parse().unwrap());
    //     ;
    // });

    // let send_thread = thread::spawn(move|| {
    //     send_test_scene("data/scene_loading/test_scene.json", "localhost:8080".parse().unwrap());
    // });

    // let test_scene = load_scene_from_network("localhost:8080".parse().unwrap());


    // send_thread.join().unwrap();
    // recv_thread.join().unwrap();

    let test_scene = scenes_and_entities::Scene::load_from_json_file("data/scene_loading/test_scene.json");

    loop {
        let test_scene = scenes_and_entities::Scene::load_from_json_file("data/scene_loading/test_scene.json");
        // let test_scene = scenes_and_entities::Scene::load_from_network("localhost:8080").unwrap();
        start_program(test_scene);
    }
    
}

// fn load_scene_from_network(addr: SocketAddr) -> scenes_and_entities::Scene {
//     let listener = TcpListener::bind(addr).unwrap();
//     match listener.accept() {
//         Ok((stream, _)) => {
//                 let packet = com::from_network(&stream);
//                 scenes_and_entities::Scene::load_from_json_str(&packet)
//         }
//         _ => {scene_composer::test_scene()},
//     }
// }

// fn send_test_scene(filepath: &str, addr: SocketAddr) {
//     let scene_packet = std::fs::read_to_string(filepath).unwrap();
//     println!("{}", scene_packet);
//     let mut stream = TcpStream::connect(addr).unwrap();
//     thread::sleep(Duration::from_millis(10));
//     stream.write_all(scene_packet.as_bytes()).unwrap();
    
// }

fn start_program(scene: scenes_and_entities::Scene) {
    // PI constants
    let pi32 = std::f32::consts::PI;
    let pi64 = std::f64::consts::PI;

    // Initialize glium items
    let event_loop = glutin::event_loop::EventLoop::new();                  // Create Event Loop
    let gui = dc::GuiContainer::init_opengl(&event_loop);                   // Initialize OpenGL interface
    info!("Initialized OpenGL items");

    let scene_ref = Arc::new(RwLock::new(scene));
    let scene_ref_2 = scene_ref.clone();
    let scene_ref_3 = scene_ref.clone();

    // Create Texture Atlas
    let (image_atlas, glyph_map) = text::load_font_atlas("/usr/share/fonts/truetype/futura/JetBrainsMono-Bold.ttf", 100.0);
    let glyph_map = Arc::new(glyph_map);
    let texture_atlas = Arc::new(create_texture_atlas(&gui.display, image_atlas));

    let text_objects: Vec<TextDisplay> = vec![
        TextDisplay::new("Hello world!".to_string(), glyph_map.clone(), texture_atlas.clone(), 0.0, 100.0),
        TextDisplay::new("DATACOM VER 0.1.0".to_string(), glyph_map.clone(), texture_atlas.clone(), 0.0, 200.0),
        TextDisplay::new((' '..='~').collect(), glyph_map.clone(), texture_atlas.clone(), 0.0, 300.0),
    ];




    // Viewport Refactor Test

    // let scale_factor = 50.0;
    let mut viewport_refactor = vec![
        dc::Twoport::new_with_camera(
            na::Point2::new(-1.0, 1.0), 
            2.0, 
            0.8*2.0, 
            scene_ref.clone(),
            na::Point3::new(-7.0, 3.0, 5.0),
            na::Point3::new(0.0, 0.0, 0.0)
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.6, 1.0), 
            0.4*2.0, 
            0.2*2.0, 
            scene_ref.clone(), 
            na::Point3::new(10.0, 0.0, 0.0),
            na::Point3::new(0.0, 0.0, 0.0)
        ),
        dc::Twoport::new_with_camera(
            na::Point2::new(0.6, 1.0-0.8), 
            2.0*(1.0-0.4), 
            0.2*2.0, 
            scene_ref.clone(), 
            na::Point3::new(0.0, 10.0, 2.0), 
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
    // viewport_refactor[1].camera.
    info!("Initialized viewports");

    // Framerate and clock items
    let frame_time_nanos = 16_666_667;
    let start_time = std::time::SystemTime::now();
    let mut t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);

    // sender Thread
    // let sender_thread = thread::Builder::new().name("sender thread".to_string()).spawn(move|| {
    //     debug!("Started data transmission thread");
    //     let addr: SocketAddr = "localhost:8080".parse().unwrap();
    //     let mut stream = TcpStream::connect(addr).unwrap();

    //     loop {
    //         t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);
    //         let test_command_data = format!("
    //             {{
    //                 \"targetEntityID\": 0,
    //                 \"commandType\": \"ComponentChangeColor\",
    //                 \"data\": [0.0,{},{},{},1.0]
    //             }}", 
    //             t.sin().abs(),
    //             t.cos().abs(),
    //             t.tan().abs()
    //         );
    //         thread::sleep(Duration::from_millis(10));
    //         stream.write_all(test_command_data.as_bytes()).unwrap();
    //         stream.flush().unwrap();
    //     }
    // });

    // Uncomment me!!
    // let listener_thread = thread::Builder::new().name("listener thread".to_string()).spawn(move || {
    //     info!("Opened listener thread");
    //     let addr: SocketAddr = "localhost:8081".parse().unwrap();
    //     com::run_server(scene_ref.clone(), addr);
    // });

    // Multithreading TRx
    // let (tx_gui, rx_gui) = mpsc::sync_channel(1);
    // Thread for calculations
    let calculation_thread = thread::Builder::new().name("calculation thread".to_string()).spawn(move || {
        info!("Started calculation thread");
        loop {
            // Clock update
            // t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);
            // tx_gui.send(t).unwrap();
            scene_ref_2.write().unwrap().update();
        }
    });

    // Draw thread
    info!("Starting event loop...");
    let mut last_key = "";
    let mut key_tracker = KeyTracker::new();
    (event_loop.run(move |event, _, control_flow| {
        let next_frame_time = std::time::Instant::now() + std::time::Duration::from_nanos(frame_time_nanos);
        // let t = rx_gui.recv().unwrap();
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
                            debug!("{}", viewport.camera.camera_position);
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
                } => {// 🔥 Pass screen dimensions from Rust
                    debug!("RIGHT");
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active {
                            viewport.move_camera(na::Vector3::<f64>::new(0.0, 1.0, 0.0));
                            debug!("{}", viewport.camera.camera_position);
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
                            debug!("{}", viewport.camera.camera_position);
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
                            debug!("{}", viewport.camera.camera_position);
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
                            debug!("{}", viewport.camera.camera_position);
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
                            debug!("{}", viewport.camera.camera_position);
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
                            debug!("{}", viewport.camera.camera_position);
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
                            debug!("{}", viewport.camera.camera_position);
                        }
                    }
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Equals),
                        ..
                    },
                    ..
                } => {
                    debug!("Last key: {}", last_key);
                    debug!("=");
                    for viewport in &mut viewport_refactor {
                        let max = viewport.content.read().unwrap().entities.len() as u64;
                        if viewport.is_active && last_key != "=" {
                            match viewport.get_target_id() {
                                Ok(id) => {
                                    
                                    if id >= max-1 {
                                        viewport.set_target_id(max-1);
                                    }
                                    else {
                                        viewport.set_target_id(id+1);
                                    }
                                    
                                },
                                _=>{}
                            }
                        }
                    }
                    last_key = "=";
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Minus),
                        ..
                    },
                    ..
                } => {
                    debug!("-");
                    
                    for viewport in &mut viewport_refactor {
                        if viewport.is_active && key_tracker.should_process_key(VirtualKeyCode::V, ElementState::Pressed) {
                            match viewport.get_target_id() {
                                Ok(id) => {
                                        if id == 0 {
                                            viewport.set_target_id(0);
                                        }
                                        else {
                                            viewport.set_target_id(id-1);
                                        }
                                    },
                                _=>{}
                            }
                        }
                    }
                    last_key = "-";
                },

                glutin::event::WindowEvent::KeyboardInput {
                    input:
                    glutin::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::V),
                        ..
                    },
                    ..
                } => {
                    if key_tracker.should_process_key(VirtualKeyCode::V, ElementState::Pressed){
                        debug!("V");
                        for viewport in &mut viewport_refactor {
                            if viewport.is_active && last_key != "v"{
                                viewport.advance_mode();
                            }
                        }
                        last_key = "v";
                    }

                    
                }, 
                _ => {last_key=""; return},
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }
        let mut current_frame = gui.display.draw();

        // current_frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
        current_frame.clear_color_and_depth((1.0, 1.0, 1.0, 1.0), 1.0);
        
        let seconds_per_rotation: f64 = 5.0;

        // viewport_refactor[0].change_camera_position(na::Point3::<f64>::new(
        //     7.0*(t/1.0).cos() as f64,
        //     7.0*(t/1.0).sin() as f64,
        //     0.0,
        // ));
        
        // Uncomment me!
        for i in &mut viewport_refactor {
            i.update_all_graphical_elements(&current_frame)
        }

        for i in &viewport_refactor {
            i.draw(&gui, &i.context, &mut current_frame)
        }

        for text_obj in &text_objects{
            text_obj.draw(&gui, &RenderContext::new_null(), &mut current_frame);
        }


        current_frame.finish().unwrap();

        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);


    }));

}

#[cfg(test)]
mod tests {
    use std::{io::Read, net::{SocketAddr, TcpListener, TcpStream}, sync::mpsc, thread};

    use crate::{dc, glutin, scene_composer, scenes_and_entities::{self, ModelComponent}};


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
            &na::Point3::<f64>::origin(),
            "Initial Position is Origin"
        );
        test_scene.get_entity(0).command(pos_cmd);
        assert_eq!(
            test_scene.get_entity(0).get_position(),
            &na::Point3::<f64>::new(1.0, 1.0, 1.0),
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

    #[test]
    fn load_font() {
        
    }
}

struct KeyTracker {
    last_press_times: HashMap<VirtualKeyCode, Instant>,
    debounce_duration: Duration,
}

impl KeyTracker {
    fn new() -> Self {
        KeyTracker {
            last_press_times: HashMap::new(),
            debounce_duration: Duration::from_millis(300),
        }
    }

    fn should_process_key(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        if state == ElementState::Pressed {
            let now = Instant::now();
            let should_process = self.last_press_times
                .get(&key)
                .map_or(true, |&last_time| 
                    now.duration_since(last_time) >= self.debounce_duration
                );

            if should_process {
                self.last_press_times.insert(key, now);
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}






// struct KeyState {
//     last_state: KeyAction,
//     last_processed_time: Instant,
// }

// impl KeyState {
//     fn new() -> Self {
//         KeyState {
//             last_state: KeyAction::Release,
//             last_processed_time: Instant::now(),
//         }
//     }
// }

// struct KeyHandler {
//     key_states: HashMap<VirtualKeyCode, KeyState>,
//     debounce_duration: Duration
// }

// impl KeyHandler {
//     fn new() -> Self {
//         KeyHandler {
//             key_states: HashMap::new(),
//             debounce_duration: Duration::from_millis(10)
//         }
//     }

//     fn should_process_key(&mut self, key: VirtualKeyCode, current_action: KeyAction) -> bool {
//         let entry = self.key_states.entry(key).or_insert(KeyState::new());

//     }
// }
// enum KeyAction {
//     Press,
//     Release
// }