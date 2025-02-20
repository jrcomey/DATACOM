use std::fmt::Error;
use std::io::{Read, Write};
// use tokio;
// use tokio::time::sleep;
use std::net::{SocketAddr, TcpStream, TcpListener};
// use std::error::Error;
use std::sync::{RwLock, Arc};
use std::time::Duration;
use std::fs::File;
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

pub fn run_server(scene_reference: Arc<RwLock<Scene>>, addr: std::net::SocketAddr) {
    info!("Server started!");
    let listener = TcpListener::bind(addr).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let packet = from_network(&stream);
                // debug!("Packet: {}", packet.as_str());
                scene_reference.write().unwrap().cmd_msg_str(&packet.as_str());
            },
            Err(_) => {}
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