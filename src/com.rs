use std::io::Read;
// use tokio;
// use tokio::time::sleep;
use std::net::{SocketAddr, TcpStream, TcpListener};
use std::error::Error;
use std::sync::{RwLock, Arc};
use std::time::Duration;

use glium::debug;

use crate::scenes_and_entities::Scene;
// use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
// use tokio::net::{TcpListener, TcpStream};

pub fn from_network(mut stream: &TcpStream) -> String{
    // debug!("Handle Commands called");
    let mut buffer = [0; 17000];
    let bytes_read = stream.read(&mut buffer).unwrap();
    let packet = String::from_utf8_lossy(&buffer[..bytes_read]);
    // println!("{}",packet);
    return packet.to_string();
}

pub fn run_server(scene_reference: Arc<RwLock<Scene>>, addr: std::net::SocketAddr) {
    let listener = TcpListener::bind(addr).unwrap();
        match listener.accept() {
            Ok((stream, _)) => {
                loop{
                    let packet = from_network(&stream);
                    scene_reference.write().unwrap().cmd_msg_str(packet.as_str());
                }
            }
            _ => {;},
        }
}