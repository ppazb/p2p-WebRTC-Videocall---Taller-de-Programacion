use opencv::core::Mat;
use opencv::highgui;
use crate::client_errors::ClientErrors;
use crate::encoding::{DecoderRtc, EncoderRtc};
use crate::mensaje::Content;
use crate::mensaje::Message;
use crate::mensaje::MsgType;
use crate::sdp_protocol::*;
use crate::video_processing;
use crate::video_processing_errors::VideoProcessingErrors;

use std::io::{Read};
use std::sync::mpsc::{self, Sender};
use std::{io, net::UdpSocket, thread};

fn main() {
    /*
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

    */

    /*
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
    */
}

#[derive(Debug)]
pub struct Client {
    socket: UdpSocket,
    peer: String,
    //sessions: Session,
    tx_local: Sender<Mat>,
    tx_remoto: Sender<Mat>
}

impl Client {
    pub fn new(my_addres: &str, peer: &str, tx_local: Sender<Mat>, tx_remoto: Sender<Mat>) -> Result<Self, ClientErrors> {
        let socket = UdpSocket::bind(my_addres).map_err(ClientErrors::CantBind)?;
        socket.connect(peer).map_err(ClientErrors::CantConect)?;

        Ok(Self {
            socket,
            peer: peer.to_string(),
            tx_local,
            tx_remoto
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

        // Thread receiver
        let tx_remoto = self.tx_remoto.clone();

        let reciver_thread = thread::spawn(move || -> Result<(), ClientErrors> {
            let mut buff = vec![0u8; 60000];
            let mut decoder = DecoderRtc::new_decoder()?;
            loop {
                match socket_recev.recv(&mut buff) {
                    Ok(n) => {
                        if n == 0 {
                            continue;
                        }
                        if let Some(msg) = Message::deserializable(&buff[..n]) {
                            println!("Received bytes: {}", n);
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
                                    println!("Received bytes (udp recv): {}", n);
                                    println!("video_encoded.len() = {}", video_encoded.len());
                                    println!("video_encoded first 12 bytes (hex): {:02x?}", &video_encoded[..video_encoded.len().min(12)]);

                                    let frame = decoder.decode(
                                        &video_encoded,
                                        msg.dimension.0,
                                        msg.dimension.1,
                                    )?;

                                    println!("frame decoded");

                                    // Cargo frame al emisor, la UI va a leerlo durante la llamada
                                    if tx_remoto.send(frame).is_err() {
                                        println!("PROBLEMA");
                                    }

                                    // highgui::imshow("Video", &frame).map_err(|_| VideoProcessingErrors::DisplayError)?;
                                    // if highgui::wait_key(1).map(|k| k == 27).unwrap_or(false) {
                                    //     break Ok(());
                                    // }
                                    println!("no me trave");
                                }
                            };
                        } else {
                            eprintln!("Received invalid message or incomplete packet ({} bytes)", n);
                        }
                    }
                    Err(e) => {
                        eprintln!("Socket recv error: {}", e);
                        continue;
                    }
                }
            }
        });

        // Thread sender
        let tx_local = self.tx_local.clone();

        let sender_thread = thread::spawn(move || -> Result<(),ClientErrors> {
            let mut camara = video_processing::initialize_camara().map_err(|_e| {
                ClientErrors::VideoProcessingError(VideoProcessingErrors::AccessCamara)
            })?;

            let camara_width = video_processing::get_frame_width(&camara)?;
            let camara_height = video_processing::get_frame_height(&camara)?;
            let mut encoder = EncoderRtc::new_encoding()?;
            loop {
                let frame = video_processing::read_frame(&mut camara).map_err(|_e| {
                    ClientErrors::VideoProcessingError(VideoProcessingErrors::CamaraSettup)
                })?;

                let encoded_frame = encoder.encode(&frame)?;

                // Cargo frame al emisor local, la UI va a leerlo cuando la cámara esté habilitada
                if tx_local.send(frame).is_err() {
                    println!("PROBLEMA");
                }

                println!("Encoded frame bytes (sender): {}", encoded_frame.len());
                println!("Encoded first 8 bytes (hex): {:02x?}", &encoded_frame[..encoded_frame.len().min(8)]);

                if encoded_frame.is_empty() { //println!("CONTINUE");
                    continue; 
                }

                let video_to_send = Message::new_msg_binary(encoded_frame, camara_width, camara_height);
                let bytes_to_send = video_to_send
                    .serialize()
                    .map_err(|_e| ClientErrors::SerializeBits)?;

                if let Err(e) = socket_send.send(&bytes_to_send) {
                    eprintln!("Send failed: {}", e);
                }

            }
        //IMPORTANTE: NO SACAR, eliminar el uso de JOIN para ""esperar"" lo thread provoca funcionamiento indeterminado
        // creemos que en principal el error de que solo un cliente pudiera estar reciviendo video venia principalmente de no estar
        // tratando los threads con un join.
        });
        // if let Err(e) = sender_thread.join() {
        //     eprintln!("Sender thread terminated: {:?}", e);
        // }

        // if let Err(e) = reciver_thread.join() {
        //     eprintln!("Receiver thread terminated: {:?}", e);
        // }

        Ok(())
        
    }
}
