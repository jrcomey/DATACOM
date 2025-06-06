use std::io::{Read, Write};
// use tokio;
// use tokio::time::sleep;
use std::net::{ToSocketAddrs, TcpStream, IpAddr, TcpListener, SocketAddr};
// use std::error::Error;
use std::sync::mpsc::Sender;
use std::{fs::File, fs, thread};
use toml::Value;
use log::{debug, info};

pub fn from_network(mut stream: &TcpStream) -> String{
    // debug!("Handle Commands called");
    let mut buffer = [0; 600000];
    match stream.read(&mut buffer){
        Ok(bytes_read) => {
            let packet = String::from_utf8_lossy(&buffer[..bytes_read]);
            // println!("{}",packet);
            packet.to_string()
        },
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            "Error: No data available yet; try again later".to_string()
        },
        Err(e) => "Error: Other".to_string(),
    }
}

// Rewrite me to be better
pub fn from_network_with_protocol(stream: &mut TcpStream) -> Result<(), &str> {
    // Get file name first, return an error if there are no more files
    const CHUNK_SIZE: usize = 4096;
    let mut name_buffer = [0; 400];
    let name_bytes_read = stream.read(&mut name_buffer).unwrap();
    let name = String::from_utf8_lossy(&name_buffer[..name_bytes_read]).to_string();
    // debug!("Name: {}", name);
    if name == "END" {
        return Err("Done");
    }
    else {
        // Open the file
        // debug!("Name: {}", name);
        let mut file = File::create(&name).unwrap();
        
        // Get file size next
        let mut file_size_buffer = [0; 8];
        stream.read_exact(&mut file_size_buffer).expect("Failed to read buffer");
        let mut bytes_received = 0;
        let file_size = u64::from_be_bytes(file_size_buffer);
        // debug!("Anticipated file size: {} bytes", file_size);

        

        let mut buffer = [0; CHUNK_SIZE];
        let mut num_packets_recieved = 0;
        while bytes_received < file_size {
            let bytes_read = stream.read(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }
            file.write_all(&buffer[..bytes_read]).unwrap();
            bytes_received += bytes_read as u64;
            // debug!("Received packet #{}", num_packets_recieved);
            num_packets_recieved += 1;
        }
        
        // Recieve file in chunks
        

    }
    Ok(())
}

pub fn run_server<A: ToSocketAddrs>(tx: Sender<String>, addr: A) {
    info!("Server started!");
    let listener = TcpListener::bind(addr).unwrap();
    info!("Connection successful!");
    listener.set_nonblocking(true).unwrap();
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(2);

    for stream in listener.incoming() {
        // info!("received TCP stream!");
        match stream {
            Ok(mut stream) => {
                stream.write_all(b"ACK").unwrap();
                stream.flush().unwrap();

                loop {
                    let packet = from_network(&stream);
                    debug!("Packet: {}", packet.as_str());
                    if !packet.starts_with("Error"){
                        // scene_reference.write().unwrap().bhvr_msg_str(&packet.as_str());
                        tx.send(packet).unwrap();
                    }
                }
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() > timeout {
                    break;
                }
            }
            Err(_) => break,
        }
    }
        // match listener.accept() {
        //     Ok((stream, _)) => {
        //         loop{
        //             let packet = from_network(&stream);
        //             scene_reference.write().unwrap().cmd_msg_str(packet.as_str());
        //         }
        //     }
        //     _ => {;},
        // }
}

pub fn get_ports(file: &str) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>>{
    let contents = fs::read_to_string(file)?;
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

pub fn create_listener_thread(tx: Sender<String>, file: String) -> Result<thread::JoinHandle<()>, std::io::Error>{
    let handle = thread::Builder::new().name("listener thread".to_string()).spawn(move || {
        info!("Opened listener thread");
        debug!("about to unwrap ports vector");
        let ports = get_ports(file.as_str()).unwrap();
        debug!("successfully unwrapped ports vector");
        let addrs_iter = &(ports[..]);
        run_server(tx, addrs_iter);
    })
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Thread spawn failed"))?;

    Ok(handle)
}

fn create_sender_thread() -> Result<thread::JoinHandle<()>, Box<dyn std::error::Error>>{
    let handle = thread::Builder::new().name("sender thread".to_string()).spawn(move|| {
        info!("Opened sender thread");
        let mut addrs_iter = "localhost:8081".to_socket_addrs().unwrap();
        let addr = addrs_iter.next().unwrap();
        let mut stream = TcpStream::connect(addr).unwrap();
        
        stream.set_nodelay(true).unwrap();
        let mut ack = [0u8; 3];
        stream.read_exact(&mut ack).unwrap();
        if &ack == b"ACK" {
            let start_time = std::time::SystemTime::now();

            loop {
                let t = (std::time::SystemTime::now().duration_since(start_time).unwrap().as_micros() as f32) / (2.0*1E6*std::f32::consts::PI);
                let test_command_data = format!("
                    {{
                        \"targetEntityID\": 0,
                        \"commandType\": \"ComponentChangeColor\",
                        \"data\": [0.0,{},{},{},1.0]
                    }}", 
                    t.sin().abs(),
                    t.cos().abs(),
                    t.tan().abs()
                );
                info!("Sending data to stream");
                thread::sleep(std::time::Duration::from_millis(10));
                stream.write_all(test_command_data.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        }
    })
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Thread spawn failed"))?;

    Ok(handle)

}

pub fn get_font() -> String{
    #[cfg(target_os="macos")]
    {
        "/Library/Fonts/Arial Unicode.ttf".to_string()
    }

    #[cfg(target_os="windows")]
    {
        "/usr/share/fonts/truetype/futura/JetBrainsMono-Bold.ttf".to_string()
    }

    #[cfg(target_os="linux")]
    {
        "/usr/share/fonts/truetype/futura/JetBrainsMono-Bold.ttf".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, net::{SocketAddr, TcpListener, TcpStream}, sync::mpsc, thread};
//     use std::io::{Write, Read};
    use std::fs::{File, OpenOptions, remove_file};

//     use crate::{dc, glutin, scene_composer, scenes_and_entities::{self, ModelComponent}};

    use super::*;
    use std::collections::HashSet;


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

    fn vectors_match(v1: Result<Vec<SocketAddr>, Box<dyn std::error::Error>>, v2: Result<Vec<SocketAddr>, Box<dyn std::error::Error>>) -> bool{
        match v1{
            Ok(_) => {},
            Err(ref e) => println!("Error msg: {e:?}"),
        };
        if v1.is_err() && v2.is_err(){
            return true;
        }
        if !(v1.is_ok() && v2.is_ok()){
            println!("returning false: case 2");
            return false;
        }

        let vec1 = v1.unwrap();
        let vec2 = v2.unwrap();

        let set1: HashSet<_> = vec1.iter().collect();
        let set2: HashSet<_> = vec2.iter().collect();
        set1 == set2
    }

    fn get_ports_template(toml_name: &str, toml_contents: &str, expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>>){
        let file_name_string = format!("{}{}", toml_name, ".toml");
        let file_name = file_name_string.as_str();
        let file_path = Path::new(file_name);
        let mut file = File::create(&file_path).unwrap();
        _ = writeln!(file, "{}", toml_contents);
        let actual = get_ports(file_name);
        assert!(vectors_match(actual, expected));
        _ = remove_file(&file_path);
    }

    #[test]
    fn get_ports_basic(){
        let toml_name = "get_ports_basic";
        let toml_contents = "[servers]
\"10.0.0.5\" = [22]";
        let expected: Result<Vec<SocketAddr>, _> = Ok(vec![SocketAddr::from(([10, 0, 0, 5], 22))]);
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_one_ip_multiple_ports(){
        let toml_name = "get_ports_one_ip_multiple_ports";
        let toml_contents = "[servers]
\"10.0.0.5\" = [22, 8080]";
        let s1 = SocketAddr::from(([10, 0, 0, 5], 22));
        let s2 = SocketAddr::from(([10, 0, 0, 5], 8080));
        let expected: Result<Vec<SocketAddr>, _> = Ok(vec![s1, s2]);
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_multiple_ip_one_port(){
        let toml_name = "get_ports_multiple_ip_one_port";
        let toml_contents = "[servers]
\"192.168.0.1\" = [443]
\"10.0.0.5\" = [22]";
        let s1 = SocketAddr::from(([192, 168, 0, 1], 443));
        let s2 = SocketAddr::from(([10, 0, 0, 5], 22));
        let expected: Result<Vec<SocketAddr>, _> = Ok(vec![s1, s2]);
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_multiple_ip_multiple_ports(){
        let toml_name = "get_ports_multiple_ip_multiple_ports";
        let toml_contents = "[servers]
\"192.168.0.1\" = [80, 443]
\"10.0.0.5\" = [22]
\"172.16.1.100\" = [21, 8080, 3000]
\"127.0.0.1\" = [8000, 8001, 8002]
\"203.0.113.42\" = [53]";
        let s1 = SocketAddr::from(([192, 168, 0, 1], 80));
        let s2 = SocketAddr::from(([192, 168, 0, 1], 443));
        let s3 = SocketAddr::from(([10, 0, 0, 5], 22));
        let s4 = SocketAddr::from(([172, 16, 1, 100], 21));
        let s5 = SocketAddr::from(([172, 16, 1, 100], 8080));
        let s6 = SocketAddr::from(([172, 16, 1, 100], 3000));
        let s7 = SocketAddr::from(([127, 0, 0, 1], 8000));
        let s8 = SocketAddr::from(([127, 0, 0, 1], 8001));
        let s9 = SocketAddr::from(([127, 0, 0, 1], 8002));
        let s10 = SocketAddr::from(([203, 0, 113, 42], 53));
        let expected: Result<Vec<SocketAddr>, _> = Ok(vec![s1, s2, s3, s4, s5, s6, s7, s8, s9, s10]);
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_localhost(){
        let toml_name = "get_ports_localhost";
        let toml_contents = "[servers]
\"localhost\" = [8081]";
        let mut addrs = "localhost:8081".to_socket_addrs().unwrap(); 
        let s1 = addrs.next().unwrap();
        let expected = Ok(vec![s1]);
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_no_server(){
        let toml_name = "get_ports_no_server";
        let toml_contents = "[somethingelse]
irrelevant = content";

        let err = "invalid = [".parse::<toml::Value>().unwrap_err();
        let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));

        // let expected: Result<Vec<SocketAddr>, _> = Ok(vec![SocketAddr::from(([10, 0, 0, 5], 22))]);
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_too_high(){
        let toml_name = "get_ports_too_high";
        let toml_contents = "[servers]
\"10.0.0.5\" = [999999999]";

        let err = "invalid = [".parse::<toml::Value>().unwrap_err();
        let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_negative(){
        let toml_name = "get_ports_negative";
        let toml_contents = "[servers]
\"10.0.0.5\" = [-1]";

        let err = "invalid = [".parse::<toml::Value>().unwrap_err();
        let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_bad_format(){
        let toml_name = "get_ports_bad_format";
        let toml_contents = "[servers]
10005 = [80]";

        let err = "invalid = [".parse::<toml::Value>().unwrap_err();
        let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
        get_ports_template(toml_name, toml_contents, expected);
    }

    #[test]
    fn get_ports_empty(){
        let toml_name = "get_ports_empty";
        let toml_contents = "[servers]";

        let err = "invalid = [".parse::<toml::Value>().unwrap_err();
        let expected: Result<Vec<SocketAddr>, Box<dyn std::error::Error>> = Err(Box::new(err));
        get_ports_template(toml_name, toml_contents, expected);
    }

    fn create_listener_thread_template(toml_name: &str, toml_contents: &str){
        pretty_env_logger::init();
        let (tx, rx) = mpsc::channel();

        let file_name_string = format!("{}{}", toml_name, ".toml");
        let file_name_string_clone = file_name_string.clone();
        let file_name = file_name_string.as_str();
        let file_path = Path::new(file_name);
        let mut file = File::create(&file_path).unwrap();
        _ = writeln!(file, "{}", toml_contents);

        let listener = create_listener_thread(tx, file_name_string_clone);
        listener.unwrap();
        let sender = create_sender_thread();
        sender.unwrap();
        // let join_result = handle.join();
        let start_time = std::time::SystemTime::now();
        let mut passed = false;
        while (std::time::SystemTime::now().duration_since(start_time).unwrap().as_secs()) < 10{
            let received = rx.recv().unwrap();
            info!("RECEIVED = {received}");
            if received.len() > 0 {
                passed = true;
                break;
            }
        }
        assert!(passed);

        _ = remove_file(&file_path);
        // join_result.unwrap();
    }

    #[test]
    fn create_listener_thread_success(){
        let toml_name = "create_listener_thread_success";
        let toml_contents = "[servers]
\"localhost\" = [8081]";
        create_listener_thread_template(toml_name, toml_contents);
    }

    #[test]
    #[should_panic]
    fn create_listener_thread_failure(){
        let toml_name = "create_listener_thread_failure";
        let toml_contents = "[somethingelse]
irrelevant = content";
        create_listener_thread_template(toml_name, toml_contents);
    }
}