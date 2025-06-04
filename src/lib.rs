use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
};
use std::net::{ToSocketAddrs, IpAddr, SocketAddr, TcpListener, TcpStream};
use std::{thread, sync::{Arc, Mutex, RwLock}};
use log::info;
use toml::Value;

mod scenes_and_entities;
mod model;
mod camera;
mod com;

fn get_ports(file: &str) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>>{
    let contents = std::fs::read_to_string(file)?;
    let parsed: Value = contents.parse::<Value>()?;
    let mut result = Vec::new();

    // get server table
    if let Some(servers) = parsed.get("servers").and_then(|v| v.as_table()) {
        // each line contains an IP address and an array of ports
        for (ip, ports) in servers {
            // println!("analyzing {ip} and {ports}");
            if let Some(port_array) = ports.as_array() {
                for port in port_array {
                    if let Some(port_num) = port.as_integer() {
                        // Convert the IP and port into a SocketAddr
                        let port: u16 = port_num.try_into()?;
                        let socket_addr: SocketAddr = if ip == "localhost" {
                            let mut addrs = format!("{}:{}", ip, port).to_socket_addrs().unwrap();
                            addrs.next().unwrap()
                        } else {
                            let ip_addr = ip.parse::<IpAddr>()?;
                            SocketAddr::new(ip_addr, port)
                        };
                        // println!("adding {ip}:{port}");
                        result.push(socket_addr);
                    }
                }
            }
        }
    }

    // we want an Err to return if no IP addresses were found
    _ = result.get(0).ok_or("No IP address was found")?;
    Ok(result)
}

fn create_listener_thread(scene_ref: Arc<RwLock<scenes_and_entities::Scene>>, file: String) -> Result<thread::JoinHandle<()>, std::io::Error>{
    let handle = thread::Builder::new().name("listener thread".to_string()).spawn(move || {
        info!("Opened listener thread");
        println!("about to unwrap ports vector");
        let ports = get_ports(file.as_str()).unwrap();
        println!("successfully unwrapped ports vector");
        let mut addrs_iter = &(ports[..]);
        com::run_server(scene_ref, addrs_iter);
    })
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Thread spawn failed"))?;

    Ok(handle)
}

pub async fn run() {                               // Initialize logger
    pretty_env_logger::init();
    info!("Program Start!");

    let event_loop = EventLoop::new().unwrap();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();

    let scene_file = "data/scene_loading/test_scene.json";

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = scenes_and_entities::State::new(&window, scene_file).await;
    let mut last_render_time = std::time::Instant::now();

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
                            state.update(dt);

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

// #[cfg(test)]
// mod tests {
//     use std::{path::Path, net::{SocketAddr, TcpListener, TcpStream}, sync::mpsc, thread};
//     use std::io::{Write, Read};
//     use std::fs::{File, OpenOptions, remove_file};

//     use crate::{dc, glutin, scene_composer, scenes_and_entities::{self, ModelComponent}};

//     use super::*;


//     #[test]
//     fn unit_quaternion() {
//         let unit_quaternion: na::UnitQuaternion<f64> = na::UnitQuaternion::identity();
//         info!("{}", unit_quaternion);
//     }

//     fn load_from_json(){
//         scenes_and_entities::ModelComponent::load_from_json_file(&"data/object_loading.blizzard_initialize.json");
        
//     }

//     #[test]
//     fn color_change() {
//         let mut test_scene = scene_composer::test_scene();
//         let color_cmd = scenes_and_entities::Command::new(
//             scenes_and_entities::CommandType::ComponentChangeColor,
//             vec![0.0, 1.0, 1.0, 1.0, 1.0]
//         );
//         assert_eq!(
//             test_scene.get_entity(0).unwrap().get_model(0).get_color(),
//             na::Vector4::<f32>::new(0.0, 1.0, 0.0, 1.0),
//             "Base color is green"
//         );
//         test_scene.get_entity(0).unwrap().command(color_cmd);
//         assert_eq!(
//             test_scene.get_entity(0).unwrap().get_model(0).get_color(),
//             na::Vector4::<f32>::new(1.0, 1.0, 1.0, 1.0),
//             "New color is white"
//         );
//     }

//     #[test]
//     fn position_change() {
//         let mut test_scene = scene_composer::test_scene();
//         let pos_cmd = scenes_and_entities::Command::new(
//             scenes_and_entities::CommandType::EntityChangePosition,
//             vec![1.0, 1.0, 1.0]
//         );
//         assert_eq!(
//             test_scene.get_entity(0).unwrap().get_position(),
//             &na::Point3::<f64>::origin(),
//             "Initial Position is Origin"
//         );
//         test_scene.get_entity(0).unwrap().command(pos_cmd);
//         assert_eq!(
//             test_scene.get_entity(0).unwrap().get_position(),
//             &na::Point3::<f64>::new(1.0, 1.0, 1.0),
//             "Position commanded successfully"
//         );
//     }

//     #[test]
//     fn change_command() {
//         let mut test_scene = scene_composer::test_scene();
//         let change_command = scenes_and_entities::Command::new(
//             scenes_and_entities::CommandType::ModifyBehavior, 
//             vec![0.0, ]
//         );
//     }

//     #[test]
//     fn load_font() {
        
//     }

//     fn vectors_match(v1: Result<Vec<SocketAddr>, Box<dyn std::error::Error>>, v2: Result<Vec<SocketAddr>, Box<dyn std::error::Error>>) -> bool{
//         match v1{
//             Ok(_) => {},
//             Err(ref e) => println!("Error msg: {e:?}"),
//         };
//         if v1.is_err() && v2.is_err(){
//             return true;
//         }
//         if !(v1.is_ok() && v2.is_ok()){
//             println!("returning false: case 2");
//             return false;
//         }

//         let vec1 = v1.unwrap();
//         let vec2 = v2.unwrap();

//         let set1: HashSet<_> = vec1.iter().collect();
//         let set2: HashSet<_> = vec2.iter().collect();
//         set1 == set2
//     }

//     fn get_ports_template(toml_name: &str, toml_contents: &str, expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>>){
//         let file_name_string = format!("{}{}", toml_name, ".toml");
//         let file_name = file_name_string.as_str();
//         let file_path = Path::new(file_name);
//         let mut file = File::create(&file_path).unwrap();
//         _ = writeln!(file, "{}", toml_contents);
//         let actual = get_ports(file_name);
//         assert!(vectors_match(actual, expected));
//         _ = remove_file(&file_path);
//     }

//     #[test]
//     fn get_ports_basic(){
//         let toml_name = "get_ports_basic";
//         let toml_contents = "[servers]
// \"10.0.0.5\" = [22]";
//         let expected: Result<Vec<SocketAddr>, _> = Ok(vec![SocketAddr::from(([10, 0, 0, 5], 22))]);
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_one_ip_multiple_ports(){
//         let toml_name = "get_ports_one_ip_multiple_ports";
//         let toml_contents = "[servers]
// \"10.0.0.5\" = [22, 8080]";
//         let s1 = SocketAddr::from(([10, 0, 0, 5], 22));
//         let s2 = SocketAddr::from(([10, 0, 0, 5], 8080));
//         let expected: Result<Vec<SocketAddr>, _> = Ok(vec![s1, s2]);
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_multiple_ip_one_port(){
//         let toml_name = "get_ports_multiple_ip_one_port";
//         let toml_contents = "[servers]
// \"192.168.0.1\" = [443]
// \"10.0.0.5\" = [22]";
//         let s1 = SocketAddr::from(([192, 168, 0, 1], 443));
//         let s2 = SocketAddr::from(([10, 0, 0, 5], 22));
//         let expected: Result<Vec<SocketAddr>, _> = Ok(vec![s1, s2]);
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_multiple_ip_multiple_ports(){
//         let toml_name = "get_ports_multiple_ip_multiple_ports";
//         let toml_contents = "[servers]
// \"192.168.0.1\" = [80, 443]
// \"10.0.0.5\" = [22]
// \"172.16.1.100\" = [21, 8080, 3000]
// \"127.0.0.1\" = [8000, 8001, 8002]
// \"203.0.113.42\" = [53]";
//         let s1 = SocketAddr::from(([192, 168, 0, 1], 80));
//         let s2 = SocketAddr::from(([192, 168, 0, 1], 443));
//         let s3 = SocketAddr::from(([10, 0, 0, 5], 22));
//         let s4 = SocketAddr::from(([172, 16, 1, 100], 21));
//         let s5 = SocketAddr::from(([172, 16, 1, 100], 8080));
//         let s6 = SocketAddr::from(([172, 16, 1, 100], 3000));
//         let s7 = SocketAddr::from(([127, 0, 0, 1], 8000));
//         let s8 = SocketAddr::from(([127, 0, 0, 1], 8001));
//         let s9 = SocketAddr::from(([127, 0, 0, 1], 8002));
//         let s10 = SocketAddr::from(([203, 0, 113, 42], 53));
//         let expected: Result<Vec<SocketAddr>, _> = Ok(vec![s1, s2, s3, s4, s5, s6, s7, s8, s9, s10]);
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_localhost(){
//         let toml_name = "get_ports_localhost";
//         let toml_contents = "[servers]
// \"localhost\" = [8081]";
//         let mut addrs = "localhost:8081".to_socket_addrs().unwrap(); 
//         let s1 = addrs.next().unwrap();
//         let expected = Ok(vec![s1]);
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_no_server(){
//         let toml_name = "get_ports_no_server";
//         let toml_contents = "[somethingelse]
// irrelevant = content";

//         let err = "invalid = [".parse::<toml::Value>().unwrap_err();
//         let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));

//         // let expected: Result<Vec<SocketAddr>, _> = Ok(vec![SocketAddr::from(([10, 0, 0, 5], 22))]);
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_too_high(){
//         let toml_name = "get_ports_too_high";
//         let toml_contents = "[servers]
// \"10.0.0.5\" = [999999999]";

//         let err = "invalid = [".parse::<toml::Value>().unwrap_err();
//         let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_negative(){
//         let toml_name = "get_ports_negative";
//         let toml_contents = "[servers]
// \"10.0.0.5\" = [-1]";

//         let err = "invalid = [".parse::<toml::Value>().unwrap_err();
//         let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_bad_format(){
//         let toml_name = "get_ports_bad_format";
//         let toml_contents = "[servers]
// 10005 = [80]";

//         let err = "invalid = [".parse::<toml::Value>().unwrap_err();
//         let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     #[test]
//     fn get_ports_empty(){
//         let toml_name = "get_ports_empty";
//         let toml_contents = "[servers]";

//         let err = "invalid = [".parse::<toml::Value>().unwrap_err();
//         let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
//         get_ports_template(toml_name, toml_contents, expected);
//     }

//     fn create_listener_thread_template(toml_name: &str, toml_contents: &str){
//         let test_scene = scenes_and_entities::Scene::load_from_json_file("data/scene_loading/test_scene.json");
//         let scene_ref = Arc::new(RwLock::new(test_scene));
//         let file_name_string = format!("{}{}", toml_name, ".toml");
//         let file_name_string_clone = file_name_string.clone();
//         let file_name = file_name_string.as_str();
//         let file_path = Path::new(file_name);
//         let mut file = File::create(&file_path).unwrap();
//         _ = writeln!(file, "{}", toml_contents);
//         let handle = create_listener_thread(scene_ref, file_name_string_clone).unwrap();
//         let join_result = handle.join();
//         _ = remove_file(&file_path);
//         join_result.unwrap();
//     }

//     #[test]
//     fn create_listener_thread_success(){
//         let toml_name = "create_listener_thread_success";
//         let toml_contents = "[servers]
// \"127.0.0.1\" = [0]";
//         create_listener_thread_template(toml_name, toml_contents);
//     }

//     #[test]
//     #[should_panic]
//     fn create_listener_thread_failure(){
//         let toml_name = "create_listener_thread_failure";
//         let toml_contents = "[somethingelse]
// irrelevant = content";
//         create_listener_thread_template(toml_name, toml_contents);
//     }
// }