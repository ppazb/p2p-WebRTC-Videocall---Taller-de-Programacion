use crate::client_errors::ClientErrors;
use crate::encoding::{DecoderRtc, EncoderRtc};
use crate::mensaje::Content;
use crate::mensaje::Message;
use crate::mensaje::MsgType;
use crate::rtp::RtpPackage;
use crate::sdp_protocol::*;
use crate::video_processing;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::core::Mat;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::{net::UdpSocket, thread};

/*
fn main() {
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
*/

#[derive(Debug)]
pub struct Client {
    socket: UdpSocket,
    //sessions: Session,
    tx_local: Sender<Mat>,
    tx_remoto: Sender<Mat>,
    in_call: Arc<AtomicBool>,
}

impl Client {
    pub fn new(
        socket: UdpSocket,
        tx_local: Sender<Mat>,
        tx_remoto: Sender<Mat>,
        in_call: Arc<AtomicBool>,
    ) -> Result<Self, ClientErrors> {
        Ok(Self {
            socket,
            tx_local,
            tx_remoto,
            in_call,
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
        let chunk_size: usize = 1200;
        let (tx, _rx) = mpsc::channel::<String>();

        // Thread receiver
        let tx_remoto = self.tx_remoto.clone();

        let in_call = Arc::clone(&self.in_call);

        let _reciver_thread = thread::spawn(move || -> Result<(), ClientErrors> {
            let mut buff = vec![0u8; 60000];
            let mut decoder = DecoderRtc::new_decoder()?;

            let mut chunks_hash: HashMap<u64, Vec<(u32, Vec<u8>)>> = HashMap::new();

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
                                            println!("\n{}", text);
                                        }
                                        _ => println!("\n {}", text),
                                    };
                                }
                                Content::Binary(video_chunk) => {
                                    if let Some(rtp) = RtpPackage::from_bytes(&video_chunk) {
                                        let time_stamp = rtp.time_stamp;
                                        let entry =
                                            chunks_hash.entry(time_stamp).or_insert_with(Vec::new);
                                        entry.push((rtp.index, rtp.payload));

                                        if entry.last().map(|(_, p)| p.len()).unwrap_or(0)
                                            < chunk_size
                                        {
                                            // reassemble
                                            entry.sort_by_key(|(i, _)| *i);
                                            let mut full: Vec<u8> = Vec::new();
                                            for (_, p) in entry.drain(..) {
                                                full.extend(p);
                                            }
                                            chunks_hash.remove(&time_stamp);

                                            match decoder.decode(
                                                &full,
                                                msg.dimension.0,
                                                msg.dimension.1,
                                            ) {
                                                Ok(frame) => {
                                                    println!("frame decoded");
                                                    if tx_remoto.send(frame).is_err() {
                                                        println!("PROBLEMA");
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "Decode error (reassembled): {:?}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        // not an RTP fragment, treat as full frame
                                        println!("Assuming full-frame payload, decoding directly");
                                        match decoder.decode(
                                            &video_chunk,
                                            msg.dimension.0,
                                            msg.dimension.1,
                                        ) {
                                            Ok(frame) => {
                                                println!("frame decoded");
                                                if tx_remoto.send(frame).is_err() {
                                                    println!("PROBLEMA");
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Decode error (full-frame): {:?}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        };
                    }
                    Err(e) => {
                        eprintln!("Socket recv error: {}", e);
                        continue;
                    }
                }

                // Controlo el flag in_call para salir del thread
                if !in_call.load(Ordering::Relaxed) {
                    println!("RECEIVER OVER");
                    return Ok(());
                }
            }
        });

        // Thread sender
        let tx_local = self.tx_local.clone();

        let in_call = Arc::clone(&self.in_call);

        let _sender_thread = thread::spawn(move || -> Result<(), ClientErrors> {
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

                // Cargo frame al emisor local, la UI va a leerlos cuando la cámara esté habilitada
                if tx_local.send(frame).is_err() {
                    println!("PROBLEMA");
                }

                println!("Encoded frame bytes (sender): {}", encoded_frame.len());
                println!(
                    "Encoded first 8 bytes (hex): {:02x?}",
                    &encoded_frame[..encoded_frame.len().min(8)]
                );

                if encoded_frame.is_empty() {
                    //println!("CONTINUE");
                    continue;
                }
                //tenemos un frame completamente econdeado
                let mut index_chunk: u32 = 0;
                for chunk in encoded_frame.chunks(chunk_size) {
                    let rtp_package = RtpPackage::new(index_chunk, chunk.to_vec());
                    let rtp_bytes = rtp_package.to_bytes();
                    let video_to_send =
                        Message::new_msg_binary(rtp_bytes, camara_width, camara_height);
                    let bytes_to_send = video_to_send
                        .serialize()
                        .map_err(|_e| ClientErrors::SerializeBits)?;
                    if let Err(e) = socket_send.send(&bytes_to_send) {
                        eprintln!("Send failed: {}", e);
                    }
                    index_chunk = index_chunk.wrapping_add(1);
                }
                // Controlo el flag in_call para salir del thread
                if !in_call.load(Ordering::Relaxed) {
                    println!("SENDER OVER");
                    return Ok(());
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
