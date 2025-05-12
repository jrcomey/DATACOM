use std::fmt::Error;
use std::io::{Read, Write};
// use tokio;
// use tokio::time::sleep;
use std::net::{ToSocketAddrs, TcpStream, IpAddr, TcpListener, SocketAddr};
// use std::error::Error;
use std::sync::{RwLock, Arc};
use std::time::Duration;
use std::{fs::File, fs, thread};
use toml::Value;
use glium::debug;

use crate::scenes_and_entities::Scene;
// use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
// use tokio::net::{TcpListener, TcpStream};

pub fn from_network(mut stream: &TcpStream) -> String{
    // debug!("Handle Commands called");
    let mut buffer = [0; 600000];
    let bytes_read = stream.read(&mut buffer).unwrap();
    let packet = String::from_utf8_lossy(&buffer[..bytes_read]);
    // println!("{}",packet);
    return packet.to_string();
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

pub fn run_server<A: ToSocketAddrs>(scene_reference: Arc<RwLock<Scene>>, addr: A) {
    info!("Server started!");
    let listener = TcpListener::bind(addr).unwrap();
    info!("Connection successful!");
    listener.set_nonblocking(true).unwrap();
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(2);

    for stream in listener.incoming() {
        // info!("received TCP stream!");
        match stream {
            Ok(stream) => {
                let packet = from_network(&stream);
                debug!("Packet: {}", packet.as_str());
                scene_reference.write().unwrap().cmd_msg_str(&packet.as_str());
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

pub fn create_listener_thread(scene_ref: Arc<RwLock<Scene>>, file: String) -> Result<thread::JoinHandle<()>, std::io::Error>{
    let handle = thread::Builder::new().name("listener thread".to_string()).spawn(move || {
        info!("Opened listener thread");
        debug!("about to unwrap ports vector");
        let ports = get_ports(file.as_str()).unwrap();
        debug!("successfully unwrapped ports vector");
        let mut addrs_iter = &(ports[..]);
        run_server(scene_ref, addrs_iter);
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