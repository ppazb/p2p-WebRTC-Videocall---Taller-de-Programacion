use opencv::highgui;
use room_rtc::client_errors::ClientErrors;
use room_rtc::encoding::{DecoderRtc, EncoderRtc};
use room_rtc::mensaje::Content;
use room_rtc::mensaje::Message;
use room_rtc::mensaje::MsgType;
use room_rtc::sdp_protocol::*;
use room_rtc::video_processing;
use room_rtc::video_processing_errors::VideoProcessingErrors;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::{io, net::UdpSocket, thread};
fn main() {

    // cargo run --bin client IP:Puerto path_sdp path_answer
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return;
    }

    let offer = match create_sdp_offer(&args[1], &args[2]) {
        Ok(session_descrip_offer) => session_descrip_offer,
        Err(e) => return eprint!("{}", e),
    };


    let mut stdin = io::stdin();
    let mut offer_client = String::new();
    let _ = stdin.read_to_string(&mut offer_client);

    let client_offer_session_descrip = match SessionDescription::parse(&offer_client) {
        Ok(md) => md,
        Err(e) => return eprintln!("{}", e), 
    }; 

    match create_sdp_answer(&offer, &client_offer_session_descrip, &args[3]) {
        Ok(_) =>  {},
        Err(e) =>  eprintln!("{}", e),
    }

    let peer_adder = client_offer_session_descrip.get_address();     
    let my_adder = offer.get_address(); 
    println!("my: {}", my_adder);
    println!("peer: {}",peer_adder);

    let client = match Client::new(&my_adder, &peer_adder) {
        Ok(client) => client,
        Err(e) => return eprintln!("{}", e),
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

        let (tx, _rx) = mpsc::channel::<String>();

        //Thread reciver
        thread::spawn(move || -> Result<(), ClientErrors> {
            let mut buff: [u8; 102400] = [0; 102400];
            let mut decoder = DecoderRtc::new_decoder()?;
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
                            let frame = decoder.decode(
                                &video_encoded,
                                msg.dimension.0,
                                msg.dimension.1,
                            )?;
                                video_processing::display_frame(&frame)?;
                                match highgui::wait_key(1) {
                                    Ok(27) => break Ok(()),
                                    Ok(_) => continue,
                                    Err(_) => break Ok(()),
                                }                                
                        }
                    };
                }

                let _ = io::stdout().flush();
            }
        });

        let mut camara = video_processing::initialize_camara().map_err(|_e| {
            ClientErrors::VideoProcessingError(VideoProcessingErrors::AccessCamara)
        })?;

        let camara_width = video_processing::get_frame_width(&camara)?;
        let camara_height = video_processing::get_frame_height(&camara)?;
        let mut encoder = EncoderRtc::new_encoding()?;
        //let mut decoder = DecoderRtc::new_decoder()?;
        loop {
            let frame = video_processing::read_frame(&mut camara).map_err(|_e| {
                ClientErrors::VideoProcessingError(VideoProcessingErrors::CamaraSettup)
            })?;

            let encoded_frame = encoder.encode(&frame)?;
            if encoded_frame.is_empty() { //println!("CONTINUE");
                continue; }
            else {
                //println!("enviando algo");
            }
                  /*/
             let decoded_frame= decoder.decode(&encoded_frame, camara_width, camara_height)?;
                video_processing::display_frame(&decoded_frame)?;
                video_processing::display_frame(&frame)?;
                match highgui::wait_key(1) {
                    Ok(27) => break Ok(()),
                    Ok(_) => (),
                    Err(_) => break Ok(()),
                }                       
                */
                let video_to_send = Message::new_msg_binary(encoded_frame, camara_width, camara_height);
                let bytes_to_send = video_to_send
                    .serialize()
                    .map_err(|_e| ClientErrors::SerializeBits)?;

                socket_send
                    .send(&bytes_to_send)
                    .map_err(|_e| ClientErrors::SendBits)?;
                /*
                if let Some(msg) = Message::deserializable(&bytes_to_send)
                {
                    match msg.content {
                        Content::Text(text) => {
                            match msg.typemsg {
                                MsgType::Signalling => {
                                    
                                }
                                _ => (),
                            };
                        }

                        Content::Binary(video_encoded) => {
                                let frame = decoder.decode(
                                &video_encoded,
                                msg.dimension.0,
                                msg.dimension.1,
                            ).unwrap();
                                video_processing::display_frame(&frame)?;
                                match highgui::wait_key(1) {
                                    Ok(27) => break Ok(()),
                                    Ok(_) => continue,
                                    Err(_) => break Ok(()),
                                }                                
                        }
                    };
                }
            */
        }
        /*
        //Thread camara visualizer
        thread::spawn(move || -> Result<(), ClientErrors> {
            let mut camara = video_processing::initialize_camara().map_err(|_e| {
                ClientErrors::VideoProcessingError(VideoProcessingErrors::AccessCamara)
            })?;

            let camara_width = video_processing::get_frame_width(&camara).unwrap();
            let camara_height = video_processing::get_frame_height(&camara).unwrap();
            let mut encoder = EncoderRtc::new_encoding(camara_width,camara_height, 500, 30);
            loop {
                let frame = video_processing::read_frame(&mut camara).map_err(|_e| {
                    ClientErrors::VideoProcessingError(VideoProcessingErrors::CamaraSettup)
                })?;



                //let mut decoder = DecoderRtc::new_decoder(640, 480, 500, 30);

                let encoded_frame = EncoderRtc::encode(&frame, & mut encoder).unwrap();

                //let decoded_frame = video_processing::decoder(&mut decoder, &encoded_frame, 640, 480);
                /*
                video_processing::display_frame(&decoded_frame).map_err(|_e| {
                    ClientErrors::VideoProcessingError(VideoProcessingErrors::DisplayError)
                })?;
                */
                let video_to_send = Message::new_msg_binary(encoded_frame, camara_width,camara_height);
                let bytes_to_send = video_to_send.serialize().unwrap();
                socket_send.send(&bytes_to_send).unwrap();
            }
        });}
        */

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
    }
}
