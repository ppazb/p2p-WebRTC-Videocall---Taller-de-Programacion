use room_rtc::mensaje::Content;
use room_rtc::mensaje::Message;
use room_rtc::room_rtc_errors::{ClientErrors,MessageErrors};
use std::io::Write;
use std::{thread,io,net::UdpSocket};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return;
    }
    let client = match Client::new(&args[1], &args[2]) {
        Ok(client) => client,
        Err(e) => return eprintln!("{}",e)
    };
    match client.initialize_call() {
        Ok(_) => (),
        Err(e) => eprintln!("{}",e)
    };
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
        let socket_send = self.socket.try_clone().map_err(ClientErrors::CantClone)?;
        let user_peer = String::from(&self.peer);
        //Thread reciver
        thread::spawn(move || {
            let mut buff = [0;1024];
            loop {
                if let Ok((n,_)) = socket_recev.recv_from(&mut buff) {
                    if let Some(msg) = Message::deserializable(&buff[..n]) {
                        match msg.content {
                            Content::Text(text) => println!("\n{}: {}", user_peer, text),
                            Content::Binary(_) => println!("Binario")
                        }
                        let _ = io::stdout().flush();
                    }
                }
            }
        });

        //thread sender
        let stdin =io::stdin();
        loop {
            io::stdout().flush().map_err(ClientErrors::CantSendMsg)?;
            let mut msg = String::new();
            if stdin.read_line(&mut msg)? == 0 {
                break;
            }
            let input = msg.trim();
            if !input.is_empty() {
                let msg_send = Message::new_msg_text(input);
                let bytes_of_message = msg_send.serialize().map_err(|_e| ClientErrors::From(MessageErrors::MsgTooLarge)) ?;
                socket_send.send(&bytes_of_message).map_err(ClientErrors::CantSendMsg)?;
            }
        };
        Ok(())

    }
}