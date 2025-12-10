use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
};
use std::sync::mpsc;
use std::collections::HashMap;
use log::{info, debug};
use std::fs::{File, remove_file};
use std::path::Path;
use std::io::{Read, Write};
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

use crate::com::{create_listener_thread, create_sender_thread};

mod scenes_and_entities;
mod state;
mod model;
mod camera;
mod com;
mod text;

const MESSAGE_TYPE_BYTE_WIDTH: usize = 2;
const FILE_ID_BYTE_WIDTH: usize = 8;
const FILE_NAME_LENGTH_BYTE_WIDTH: usize = 1;
const FILE_LENGTH_BYTE_WIDTH: usize = 4;
const FILE_METADATA_BYTE_WIDTH: usize = MESSAGE_TYPE_BYTE_WIDTH + FILE_ID_BYTE_WIDTH + FILE_NAME_LENGTH_BYTE_WIDTH + FILE_LENGTH_BYTE_WIDTH;

const CHUNK_OFFSET_BYTE_WIDTH: usize = 8;
const CHUNK_LENGTH_BYTE_WIDTH: usize = 4;
const CHUNK_METADATA_BYTE_WIDTH: usize = MESSAGE_TYPE_BYTE_WIDTH + FILE_ID_BYTE_WIDTH + CHUNK_OFFSET_BYTE_WIDTH + CHUNK_LENGTH_BYTE_WIDTH;
const FILE_END_METADATA_BYTE_WIDTH: usize = MESSAGE_TYPE_BYTE_WIDTH + FILE_ID_BYTE_WIDTH;

const SECONDS_UNTIL_TIMEOUT: u64 = 10;

#[repr(u16)]
enum MessageType {
    FILE_START,
    FILE_CHUNK,
    FILE_END,
    FILE_ACK,
    ERROR,
}

impl MessageType {
    fn get_from_bytes(value: u16) -> Self {
        match value {
            0 => MessageType::FILE_START,
            1 => MessageType::FILE_CHUNK,
            2 => MessageType::FILE_END,
            3 => MessageType::FILE_ACK,
            _ => MessageType::ERROR,
        }
    }
}

struct ActiveTransferFile {
    id: u64,
    name: String,
    length: u32,
    data: Box<[u8]>,
}

pub async fn run_scene_from_hdf5(args: Vec<String>, should_save_to_file: bool) {
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

    let mut state = state::State::new(&window, scene_file).await;
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
                    state.viewports[0].camera_controller.process_mouse(delta.0, delta.1)
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
                            // println!("dt = {}", dt.as_millis());
                            last_render_time = now;
                            state.update(dt, should_save_to_file);

                            match state.render(should_save_to_file) {
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
    debug!("Running lib.rs::run_scene_from_json()");

    let (tx, rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel();

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
    let mut state = state::State::new(&window, scene_file).await;
    let mut last_render_time = std::time::Instant::now();

    com::create_listener_thread(tx, "cargo/config.toml".to_string()).unwrap();

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion{ delta, },
                    .. // We're not using device_id currently
                } => if state.mouse_pressed {
                    state.viewports[0].camera_controller.process_mouse(delta.0, delta.1)
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

                            match state.render(false) {
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
                let msg_str = String::from_utf8(message).unwrap();
                info!("message received from listener thread: {msg_str}");
            }
        })
        .unwrap();
}

pub async fn run_scene_from_network(args: Vec<String>){
    debug!("Running lib.rs::run_scene_from_network()");

    let toml_name = "ports";
    let file_name_string = format!("{}{}", toml_name, ".toml");
    let file_name_string_clone = file_name_string.clone();
    let file_name = file_name_string.as_str();
    let file_path = Path::new(file_name);
    let mut file = File::create(&file_path).unwrap();
    let ports_str = "[servers]
    \"localhost\" = [8081]";
    _ = writeln!(file, "{}", ports_str);

    let (tx, rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel();
    let listener_result = create_listener_thread(tx, file_name_string_clone);
    let listener = listener_result.unwrap();
    let sender_result = create_sender_thread();
    let sender = sender_result.unwrap();

    let mut active_files: HashMap<u64, ActiveTransferFile> = HashMap::new();
    
    loop {
        debug!("active files len = {}", active_files.len());
        receive_file(&rx, &mut active_files);

        if active_files.is_empty(){
            break;
        }
    }

    debug!("Loop is over; active files must be empty");

    _ = remove_file(&file_path);

    run_scene_from_json(args).await;

    listener.join().unwrap();
    debug!("Listener thread closed");
    sender.join().unwrap();
    debug!("Sender thread closed");
}

fn has_timed_out(start_time: SystemTime) -> bool {
    std::time::SystemTime::now().duration_since(start_time).unwrap().as_secs() >= SECONDS_UNTIL_TIMEOUT
}

fn receive_file_metadata(rx: &Receiver<Vec<u8>>, buf: &mut Vec<u8>, start_time: SystemTime) -> ActiveTransferFile {
    while buf.len() < MESSAGE_TYPE_BYTE_WIDTH + FILE_ID_BYTE_WIDTH + FILE_NAME_LENGTH_BYTE_WIDTH && !has_timed_out(start_time){
        let msg = rx.recv().unwrap();
        buf.extend_from_slice(&msg);
    }

    // println!("in listener thread:");
    // for &byte in buf.iter() {
    //     print!("{byte} ");
    // }
    println!();
    let mut counter = MESSAGE_TYPE_BYTE_WIDTH;
    let id_bytes: [u8; FILE_ID_BYTE_WIDTH] = buf[counter..counter + FILE_ID_BYTE_WIDTH]
        .try_into()
        .expect("file ID is incorrect size");
    counter += FILE_ID_BYTE_WIDTH;

    let name_length_bytes: [u8; FILE_NAME_LENGTH_BYTE_WIDTH] = buf[counter..counter + FILE_NAME_LENGTH_BYTE_WIDTH]
        .try_into()
        .expect("name length is incorrect size");
    let name_length = u8::from_ne_bytes(name_length_bytes);
    let name_length_usize = name_length as usize;
    counter += FILE_NAME_LENGTH_BYTE_WIDTH;

    while buf.len() < MESSAGE_TYPE_BYTE_WIDTH + FILE_END_METADATA_BYTE_WIDTH + FILE_NAME_LENGTH_BYTE_WIDTH + name_length_usize {
        let msg = rx.recv().unwrap();
        buf.extend_from_slice(&msg);
    }

    let name: Vec<u8> = buf[counter..counter + name_length_usize].to_vec();
    counter += name_length_usize;

    debug!("indexing from {} to {}", counter, counter+FILE_LENGTH_BYTE_WIDTH);
    let length_bytes: [u8; FILE_LENGTH_BYTE_WIDTH] = buf[counter..counter + FILE_LENGTH_BYTE_WIDTH]
        .try_into().expect("file length is incorrect length");
    let length = u32::from_ne_bytes(length_bytes);

    let _ = buf.drain(0..FILE_METADATA_BYTE_WIDTH+name_length_usize);

    debug!("file ID bytes = {:?}", id_bytes);
    let id = u64::from_ne_bytes(id_bytes);
    debug!("file ID = {id}");
    assert!(id == 0123456789u64);

    // debug!("file name length")

    debug!("file length bytes = {:?}", length_bytes);
    debug!("file length = {length}");
    assert!(length == 12008u32);

    ActiveTransferFile {
        id: u64::from_ne_bytes(id_bytes),
        name: String::from_utf8(name).unwrap(),
        length,
        data: vec![0u8; length as usize].into_boxed_slice(),
    }

    // metadata_buf[0..overflow_len].copy_from_slice(&overflow);
    // if overflow_len >= FILE_METADATA_BYTE_WIDTH {
    //     return metadata_buf;
    // }

    // overflow.clear();
    // let mut counter = overflow_len;
    // while counter < FILE_METADATA_BYTE_WIDTH && !has_timed_out(start_time){
    //     let msg_str = rx.recv().unwrap();
    //     let msg = msg_str.as_bytes();
    //     let msg_len = msg.len();
    //     if counter + msg_len > FILE_METADATA_BYTE_WIDTH {
    //         metadata_buf[counter..FILE_METADATA_BYTE_WIDTH].copy_from_slice(&msg[counter..FILE_METADATA_BYTE_WIDTH - counter]);
    //         overflow.extend_from_slice(&msg[FILE_METADATA_BYTE_WIDTH - counter..msg_len]);
    //     } else {
    //         metadata_buf[counter..counter + msg_len].copy_from_slice(&msg);
    //     }
    //     info!("RECEIVED = {:?}", msg);
    //     counter += msg_len;
    // }

    // metadata_buf
}

fn receive_file_chunk(rx: &Receiver<Vec<u8>>, buf: &mut Vec<u8>, start_time: SystemTime, active_files: &mut HashMap<u64, ActiveTransferFile>){
    while buf.len() < CHUNK_METADATA_BYTE_WIDTH && !has_timed_out(start_time){
        let msg = rx.recv().unwrap();
        buf.extend_from_slice(&msg);
    }

    debug!("received chunk metadata");

    let mut counter = MESSAGE_TYPE_BYTE_WIDTH;
    let file_id_bytes: [u8; FILE_ID_BYTE_WIDTH] = buf[counter..counter+FILE_ID_BYTE_WIDTH]
        .try_into().expect("file ID is incorrect length");
    counter += FILE_ID_BYTE_WIDTH;
    let chunk_offset_bytes: [u8; CHUNK_OFFSET_BYTE_WIDTH] = buf[counter..counter+CHUNK_OFFSET_BYTE_WIDTH]
        .try_into().expect("chunk offset is incorrect length");
    counter += CHUNK_OFFSET_BYTE_WIDTH;
    let chunk_length_bytes: [u8; CHUNK_LENGTH_BYTE_WIDTH] = buf[counter..counter+CHUNK_LENGTH_BYTE_WIDTH]
        .try_into().expect("chunk length is incorrect length");

    debug!("parsed chunk metadata");

    let file_id = u64::from_ne_bytes(file_id_bytes);
    let chunk_offset = u64::from_ne_bytes(chunk_offset_bytes) as usize;
    let chunk_length = u32::from_ne_bytes(chunk_length_bytes) as usize;
    info!("file ID = {}, chunk offset = {}, chunk length = {}", file_id, chunk_offset, chunk_length);

    let file_data = active_files.get_mut(&file_id).expect("invalid file");

    debug!("retrieved file data");
    debug!("while {} < {}", buf.len(), CHUNK_METADATA_BYTE_WIDTH+chunk_length);
    
    while buf.len() < CHUNK_METADATA_BYTE_WIDTH+(chunk_length as usize) && !has_timed_out(start_time){
        let msg = rx.recv().unwrap();
        buf.extend_from_slice(&msg);
    }

    debug!("received chunk payload");

    file_data.data[chunk_offset..chunk_offset+chunk_length].copy_from_slice(&buf[CHUNK_METADATA_BYTE_WIDTH..CHUNK_METADATA_BYTE_WIDTH+chunk_length]);
    buf.drain(0..CHUNK_METADATA_BYTE_WIDTH+chunk_length);


    // loop {
    //     let msg_str = rx.recv().unwrap();
    //     let msg = msg_str.as_bytes();
    //     let msg_len = msg.len();
    //     buf.extend_from_slice(msg);
    //     if counter >= MESSAGE_TYPE_BYTE_WIDTH + FILE_LENGTH_BYTE_WIDTH {
    //         let file_len_bytes: [u8; 4] = buf[MESSAGE_TYPE_BYTE_WIDTH..MESSAGE_TYPE_BYTE_WIDTH+FILE_LENGTH_BYTE_WIDTH]
    //         .try_into()
    //         .expect("file length must be exactly 4 bytes");
    //         let file_len = u32::from_ne_bytes(file_len_bytes);

    //     }

    //     counter += msg_len;

    //     if buf.len() >= metadata_length + chunk_length {
    //         break;
    //     }
    // }

    // TODO: write to file
    /*
    
        each chunk begins with:
        2B message type
        8B file id
        8B chunk offset
        4B chunk length
        XB payload

        metadata len = <const>
        if buf len > metadata len + chunk len
            break
     */
}

fn finish_receiving_file(rx: &Receiver<Vec<u8>>, buf: &mut Vec<u8>, start_time: SystemTime, active_files: &mut HashMap<u64, ActiveTransferFile>){
    while buf.len() < FILE_END_METADATA_BYTE_WIDTH && !has_timed_out(start_time){
        let msg = rx.recv().unwrap();
        buf.extend_from_slice(&msg);        
    }

    let file_id_bytes: [u8; FILE_ID_BYTE_WIDTH] = buf[MESSAGE_TYPE_BYTE_WIDTH..MESSAGE_TYPE_BYTE_WIDTH+FILE_ID_BYTE_WIDTH]
        .try_into().expect("file ID is incorrect length");
    let file_id = u64::from_ne_bytes(file_id_bytes);
    let file_data = active_files.remove(&file_id).unwrap();
    let path = Path::new(&file_data.name);
    let mut file = File::create(path).unwrap();
    let file_contents = String::from_utf8(file_data.data.into_vec()).unwrap();
    let _ = writeln!(file, "{}", file_contents.as_str());
}

fn receive_file(rx: &Receiver<Vec<u8>>, active_files: &mut HashMap<u64, ActiveTransferFile>){
    debug!("Preparing to receive file");
    let start_time = std::time::SystemTime::now();
    /*
        file transfer begins with:
        2B message type = FILE_START
        8B file_id
        4B file_size
        filename
        sha256

        strategy:
        once all chunks have been received (known by comparing bytes received against file length)
            validate data
            use the data (write to file, etc)
     */

    // create an array with the length required to hold all the metadata
    // read in bytes until the metadata array is full
    /*
    let metadata len = 16
    let counter = 12
    let received = 10
    we want to place bytes 0-3 inclusive into metadata 12-15
    and bytes 4-9 into overflow

    start with 2-byte buffer
    read until buffer filled
    translate buffer to MessageType
    use match statement to handle the rest of the data
    if msg type is not FILE START and metadata has not been filled out, throw error
    if msg type is FILE START:
        find metadata
        create file struct, containing metadata and a buf of the file info transferred so far
        store metadata in hash map of active files
    else if msg type is FILE CHUNK:
        read until we have file ID
        check if ID matches active file
            if not, throw error
        read rest of metadata (chunk offset, chunk length)
        read chunk_length amount more data
        store that chunk in the correct location in the file struct
    else if msg type is FILE END:
        check if all the chunks were read in
        write to file
        remove file from active transfers list
     */

    let mut bytes_read = 0usize;
    let mut buf: Vec<u8> = Vec::new();
    while bytes_read < MESSAGE_TYPE_BYTE_WIDTH && !has_timed_out(start_time) {
        let msg = rx.recv().unwrap();
        println!("read in {:?}", msg);
        let msg_len = msg.len();
        buf.extend_from_slice(&msg);
        bytes_read += msg_len;
    }


    debug!("found message type");


    // let mut overflow: Vec<u8> = Vec::new();
    // while counter < MESSAGE_TYPE_BYTE_WIDTH && !has_timed_out(start_time){
    //     let msg_str = rx.recv().unwrap();
    //     let msg = msg_str.as_bytes();
    //     let msg_len = msg.len();
    //     if counter + msg_len > MESSAGE_TYPE_BYTE_WIDTH {
    //         message_type_buf[counter..MESSAGE_TYPE_BYTE_WIDTH].copy_from_slice(&msg[counter..MESSAGE_TYPE_BYTE_WIDTH-counter]);
    //         overflow.extend_from_slice(&msg[MESSAGE_TYPE_BYTE_WIDTH-counter..msg_len]);
    //     } else {
    //         message_type_buf[counter..counter+msg_len].copy_from_slice(&msg);
    //     }

    //     counter += msg_len;
    // }

    let message_type = MessageType::get_from_bytes(
        u16::from_ne_bytes(
            buf[0..MESSAGE_TYPE_BYTE_WIDTH]
            .try_into()
            .unwrap()
        )
    );
    
    match message_type {
        MessageType::FILE_START => {
            debug!("received FILE_START");
            let file = receive_file_metadata(&rx, &mut buf, start_time);
            active_files.insert(file.id, file);
        },
        MessageType::FILE_CHUNK => {
            debug!("received FILE_CHUNK");
            receive_file_chunk(&rx, &mut buf, start_time, active_files);
        },
        MessageType::FILE_END => {
            debug!("received FILE_END");
            finish_receiving_file(&rx, &mut buf, start_time, active_files);
        },
        MessageType::FILE_ACK => {
            debug!("received FILE_ACK");
        },
        MessageType::ERROR => {
            debug!("received ERROR");
        },
    };

    // debug!("File metadata complete. Ready to read file chunks...");

    // let file_len_as_bytes: [u8; FILE_LENGTH_BYTE_WIDTH] = metadata[0..FILE_LENGTH_BYTE_WIDTH].try_into().unwrap();
    // let file_len = u32::from_ne_bytes(file_len_as_bytes);

    // validate the metadata and send a message to the server
        
        
    // create an N-byte array
    // let mut chunk_metadata_arr = [0u8; CHUNK_METADATA_BYTE_WIDTH];
    // let mut data_vec = vec![0u8; file_len as usize];

    // // for each chunk received:
    //     // validate that the file ID matches an active file transfer
    //     // fill in the corresponding part of the array using the chunk offset and length
    // let mut chunk_metadata_counter: usize = 0;
    // let mut overflow: Vec<u8> = Vec::new();
    // while chunk_metadata_counter < CHUNK_METADATA_BYTE_WIDTH && !has_timed_out(start_time){
    //     let received_str = rx.recv().unwrap();
    //     let received = received_str.as_bytes();
    //     let received_len = received.len();
    //     if chunk_metadata_counter + received_len > CHUNK_METADATA_BYTE_WIDTH {
    //         overflow.extend_from_slice(&received[0..CHUNK_METADATA_BYTE_WIDTH - chunk_metadata_counter]);
    //     }
    //     chunk_metadata_arr[chunk_metadata_counter..chunk_metadata_counter+received_len].copy_from_slice(received);
    //     info!("RECEIVED = {received_str}");
    //     chunk_metadata_counter += received_len;
    // }

    // let mut i = 0;
    // let message_type = MessageType::get_from_bytes(
    //     u16::from_ne_bytes(
    //         chunk_metadata_arr[i..i+MESSAGE_TYPE_BYTE_WIDTH]
    //         .try_into()
    //         .unwrap()
    //     )
    // );
    // i += MESSAGE_TYPE_BYTE_WIDTH;
    // let file_id = u64::from_ne_bytes(chunk_metadata_arr[i..i+FILE_ID_BYTE_WIDTH].try_into().unwrap());
    // i += FILE_ID_BYTE_WIDTH;
    // let chunk_offset = u64::from_ne_bytes(chunk_metadata_arr[i..i+CHUNK_OFFSET_BYTE_WIDTH].try_into().unwrap());
    // i += CHUNK_OFFSET_BYTE_WIDTH;
    // let chunk_length = u32::from_ne_bytes(chunk_metadata_arr[i..i+CHUNK_LENGTH_BYTE_WIDTH].try_into().unwrap());

    // let mut j = 0usize;
    // while j < chunk_length as usize && !has_timed_out(start_time){
    //     let received_str = rx.recv().unwrap();
    //     let received = received_str.as_bytes();
    //     let received_len = received.len();
    //     let data_vec_index = chunk_offset as usize + j;
    //     data_vec[data_vec_index..data_vec_index+received_len].copy_from_slice(received);
    //     j += received_len;
    // }

    info!("DONE");
        
}