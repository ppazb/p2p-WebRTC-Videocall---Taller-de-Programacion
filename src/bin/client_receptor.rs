use opencv::highgui;
use room_rtc::mensaje::Content;
use room_rtc::mensaje::Message;
use room_rtc::mensaje::MsgType;
use room_rtc::encoding::{encoder_rtc,decoder_rtc};
use room_rtc::room_rtc_errors::{ClientErrors, MessageErrors};
use room_rtc::sdp_protocol::*;
use room_rtc::video_processing;
use std::io::Write;
use std::sync::mpsc;
use std::{io, net::UdpSocket, thread};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return;
    }
    let client = match Client::new(&args[1], &args[2]) {
        Ok(client) => client,
        Err(e) => return eprintln!("{}", e),
    };

    let _offer = match create_sdp_offer(&args[1], &args[3]) {
        Ok(session_descrip_offer) => session_descrip_offer,
        Err(e) => return eprint!("{}", e),
    };

    match client.initialize_call() {
        Ok(_) => (),
        Err(e) => eprintln!("{}", e),
    };
}

#[derive(Debug)]
pub struct Client {
    socket: UdpSocket,
    peer: String,
    //sessions: Session,
}

impl Client {
    pub fn new(my_addres: &str, peer: &str) -> Result<Self, ClientErrors> {
        let socket = UdpSocket::bind(my_addres).map_err(ClientErrors::CantBind)?;
        socket.connect(peer).map_err(ClientErrors::CantConect)?;
        Ok(Self {
            socket,
            peer: peer.to_string(),
        })
    }

    pub fn handle_signalling(&self, text: String) {
        let peer_session_description = match SessionDescription::parse(&text) {
            Ok(peer_sd) => peer_sd,
            Err(e) => return eprintln!("{}", e),
        };

        println!("[Session Description] {}", peer_session_description);
    }

    pub fn initialize_call(&self) -> Result<(), ClientErrors> {
        let socket_recev = self.socket.try_clone().map_err(ClientErrors::CantClone)?;
        let socket_send = self.socket.try_clone().map_err(ClientErrors::CantClone)?;
        let user_peer = String::from(&self.peer);

        let (tx, rx) = mpsc::channel::<String>();

        //Thread reciver
        thread::spawn(move || {
            let mut buff: [u8; 102400] = [0; 102400];
            let mut decoder = decoder_rtc::new_decoder(640, 480, 30, 30);
            loop {
                if let Ok((n, _)) = socket_recev.recv_from(&mut buff)
                    && let Some(msg) = Message::deserializable(&buff[..n])
                {
                    match msg.content {
                        Content::Text(text) => {
                            match msg.typemsg {
                                MsgType::Signalling => {
                                    let _ = tx.send(text.to_string());
                                    println!("\n{}: {}", user_peer, text);
                                }
                                _ => println!("\n{}: {}", user_peer, text),
                            };
                        }

                        Content::Binary(video_encoded) => {
                            let frame =  decoder_rtc::decoder(&mut decoder, &video_encoded, 640, 480);
                            video_processing::display_frame(& frame).unwrap();
                            match highgui::wait_key(1) {
                            Ok(27) => break,
                            Ok(_) => continue,
                            Err(_) => break,
                            }
                        },
                    };
                }

                let _ = io::stdout().flush();
            }
        });

        loop {
            
        }
/*
        //thread sender
        let stdin = io::stdin();
        loop {
            if let Ok(signal_text) = rx.try_recv() {
                self.handle_signalling(signal_text);
            }

            io::stdout().flush().map_err(ClientErrors::CantSendMsg)?;
            let mut msg = String::new();
            if stdin.read_line(&mut msg)? == 0 {
                break;
            }
            let input = msg.trim();
            if !input.is_empty() {
                // let msg_send: Message;
                // if input.starts_with("/offer") {

                //     if let Some(input) = input.strip_prefix("/offer\n") {
                //         msg_send = Message::new_msg_signalling(input);
                //     } else {
                //         msg_send = Message::new_msg_text(input);
                //     };

                // } else {
                //     msg_send = Message::new_msg_text(input)
                // }
                let msg_send = Message::new_msg_text(input);
                let bytes_of_message = msg_send
                    .serialize()
                    .map_err(|_e| ClientErrors::MessageError(MessageErrors::MsgTooLarge))?;
                socket_send
                    .send(&bytes_of_message)
                    .map_err(ClientErrors::CantSendMsg)?;
            }
        }
        */
        Ok(())
    }
}
