use room_rtc::room_rtc_errors::ClientErrors;
use room_rtc::mensaje;
use room_rtc::room_rtc_errors;
use std::{thread,io,net::UdpSocket};
fn main() {
    println!("Hola soy cliente");
    let c = Client::new("hola","hola");
}




#[derive(Debug)]
pub struct Client {
    socket:UdpSocket,
    peer:String
}
impl Client {
    pub fn new(my_addres: &str, peer: &str) -> Result<Self,ClientErrors> {
        let socket = UdpSocket::bind(my_addres).map_err(ClientErrors::CantBind)?;
        socket.connect(peer).map_err(ClientErrors::CantConect)?;
        Ok(Self {
            socket: socket,
            peer:peer.to_string()
        })
    }
    pub fn initialize_call(&self) -> Result<(),ClientErrors> {
        let socket_recev = self.socket.try_clone().map_err(ClientErrors::CantClone)?;
        thread::spawn(move || {
            let mut buff = [0;1024];
            loop {
                if let Ok((n,_)) = socket_recev.recv_from(&mut buff) {
                    if let Some(msg) = mensaje
                }
            }
        });
        Ok(())

    }
}