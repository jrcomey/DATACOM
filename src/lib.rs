use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
};
use std::sync::mpsc;
use log::info;

mod scenes_and_entities;
mod model;
mod camera;
mod com;
mod text;

pub async fn run_scene_from_hdf5(args: Vec<String>, should_save_to_file: bool) {
    pretty_env_logger::init();
    info!("Program Start!");

    let event_loop = EventLoop::new().unwrap();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();

    let mut scene_file_string = String::from("data/");
    scene_file_string.push_str(&args[1]);
    let scene_file = scene_file_string.as_str();

    let mut state = scenes_and_entities::State::new(&window, scene_file).await;
    let mut last_render_time = std::time::Instant::now();

    // State::update() calls scene.run_behaviors(), which calls entity.run_behaviors() on every entity
    // in the JSON implementation, objects get their existence and behaviors from JSONs
    // in this implementation, objects get their existence from the Vehicles section
    // and their behaviors from the data below
    /*
        our data is divided into timesteps and data point (pos, rot)
        we want to run state.update() every timestep
        we want every timestep to run in time (eg timestep 1.002 should occur 1.002 seconds after starting)
        step 1: figure out how to make a loop run every timestep
            start = now
            func()
            end = now
            sleep(timestep - (end-start))

        step 2: figure out how to get each entity its data
            lib.rs has the data
            State->Scene->Entity
            state.update()
                scene.run_behaviors()
                    entity.run_behaviors()
            give each entity its data
        
        root->Vehicles->vehicle_name (eg Blizzard_0)

        step 3: construct an initial scene given HDF5 info
            lib.rs::run_scene_from_hdf5 takes in a filepath
            runs scenes_and_entities::State::new(&window, filepath, filetype)
            if filetype = hdf5:
                run Scene::load_scene_from_hdf5(filepath, &device, &model_bind_group_layout)
        
        for entity in scene.entities:
            entity.set_pos(scene.pos_data[entity_index][timestep])
            
     */

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion{ delta, },
                    .. // We're not using device_id currently
                } => if state.mouse_pressed {
                    state.camera_controller.process_mouse(delta.0, delta.1)
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.window().id() && !state.input(event) => {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            // This tells winit that we want another frame after this one
                            state.window().request_redraw();
                            let now = std::time::Instant::now();
                            let dt = now - last_render_time;
                            println!("dt = {}", dt.as_millis());
                            last_render_time = now;
                            state.update(dt, should_save_to_file);

                            match state.render() {
                                Ok(_) => {}
                                // Reconfigure the surface if it's lost or outdated
                                Err(
                                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                ) => state.resize(state.size),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                                    log::error!("OutOfMemory");
                                    control_flow.exit();
                                }

                                // This happens when the a frame takes too long to present
                                Err(wgpu::SurfaceError::Timeout) => {
                                    log::warn!("Surface timeout")
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .unwrap();
}


pub async fn run_scene_from_json(args: Vec<String>) {
    pretty_env_logger::init();
    info!("Program Start!");

    let (tx, rx) = mpsc::channel();

    let event_loop = EventLoop::new().unwrap();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();

    let mut scene_file_string = String::from("data/scene_loading/");
    scene_file_string.push_str(&args[1]);
    let scene_file = scene_file_string.as_str();

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = scenes_and_entities::State::new(&window, scene_file).await;
    let mut last_render_time = std::time::Instant::now();

    com::create_listener_thread(tx, "cargo/config.toml".to_string()).unwrap();

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion{ delta, },
                    .. // We're not using device_id currently
                } => if state.mouse_pressed {
                    state.camera_controller.process_mouse(delta.0, delta.1)
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.window().id() && !state.input(event) => {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            // This tells winit that we want another frame after this one
                            state.window().request_redraw();
                            let now = std::time::Instant::now();
                            let dt = now - last_render_time;
                            last_render_time = now;
                            state.update(dt, false);

                            match state.render() {
                                Ok(_) => {}
                                // Reconfigure the surface if it's lost or outdated
                                Err(
                                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                ) => state.resize(state.size),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                                    log::error!("OutOfMemory");
                                    control_flow.exit();
                                }

                                // This happens when the a frame takes too long to present
                                Err(wgpu::SurfaceError::Timeout) => {
                                    log::warn!("Surface timeout")
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            while let Ok(message) = rx.try_recv() {
                info!("message received from listener thread: {message}");
            }
        })
        .unwrap();
}

pub async fn run_scene_from_network(args: Vec<String>){
    run_scene_from_json(args).await;
}