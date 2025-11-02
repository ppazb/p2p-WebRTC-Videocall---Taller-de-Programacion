use crate::client_errors::ClientErrors;
use crate::encoding::{DecoderRtc, EncoderRtc};
use crate::mensaje::Message;
use crate::mensaje::MsgType;
use crate::mensaje::{Content, Signal};
use crate::mensaje_errors::MessageErrors;
use crate::rtp::RtpPackage;
use crate::sdp_protocol::*;
use crate::video_processing;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::core::Mat;
use opencv::videoio::VideoCapture;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Duration;
use std::vec;
use std::{net::UdpSocket, thread};

#[derive(Debug)]
pub struct Client {
    socket_send: UdpSocket,
    socket_recev:UdpSocket,
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
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        Ok(Self {
            socket_recev: socket.try_clone().map_err(ClientErrors::CantClone)?,
            socket_send:socket,
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

    pub fn initialize_call(
        &self,
    ) -> Result<Vec<JoinHandle<Result<(), ClientErrors>>>, ClientErrors> {
        let socket_recev = self.socket_recev.try_clone().map_err(ClientErrors::CantClone)?;
        let socket_send = self.socket_send.try_clone().map_err(ClientErrors::CantClone)?;
        // Thread receiver
        let tx_remoto = self.tx_remoto.clone();
        let in_call = Arc::clone(&self.in_call);

        let receiver_thread = 
        thread::spawn(move || -> Result<(), ClientErrors> {Self::reciver(&socket_recev, &tx_remoto, &in_call)});

        // Thread sender
        let tx_local = self.tx_local.clone();
        let in_call = Arc::clone(&self.in_call);

        let sender_thread = 
        thread::spawn(move || -> Result<(), ClientErrors> {Self::sender(&socket_send, &tx_local, &in_call)});
        let handles = vec![receiver_thread, sender_thread];
        Ok(handles)
    }

    fn sender(socket_send: & UdpSocket,tx_local: & Sender<Mat>,in_call: & Arc<AtomicBool>) -> Result<(),ClientErrors> {
        let mut camara = video_processing::initialize_camara().map_err(|_e| {
            ClientErrors::VideoProcessingError(VideoProcessingErrors::AccessCamara)
        })?;
        let mut encoder = EncoderRtc::new_encoding()?;

        loop {
            // Corte de llamada y notificación remota
            if !in_call.load(Ordering::SeqCst) {
                // Genero el mensaje y lo envío repetidas veces para asegurarlo
                let msg = Message::new_msg_signalling(Signal::EndCall);

                for _ in 0..3 {
                    if let Ok(bytes) = msg.serialize() {
                        if socket_send.send(&bytes).is_err() {
                            eprintln!("Error enviando señal EndCall");
                        } else {
                            println!("Señal EndCall enviada");
                        }
                    }
                }
                println!("SENDER OVER");
                return Ok(());
            }
            Self::send_message(&socket_send, &mut camara, &mut encoder, &tx_local)?;
        }
        }

    fn reciver(socket_recev: & UdpSocket,tx_remoto: & Sender<Mat>,in_call: & Arc<AtomicBool>) ->Result<(),ClientErrors> {
        let mut decoder = DecoderRtc::new_decoder()?;
        let mut chunks_hash: HashMap<u64, Vec<(u32, Vec<u8>)>> = HashMap::new();
        loop {
            // Controlo el flag in_call para salir del thread
            if !in_call.load(Ordering::SeqCst) {
                println!("RECEIVER OVER");
                return Ok(());
            }
            if let Some(msg) = Self::recieve_message(& socket_recev) {
                //Self::process_content(msg,&mut decoder,&mut chunks_hash,& tx_remoto);
                match msg.content {
                    Content::Signal(signal) => Self::signal_handle(signal,& in_call),
                    Content::Text(text) => continue,
                    Content::Binary(media) => Self::video_handle(media,&mut decoder,&mut chunks_hash, msg.dimension.0,msg.dimension.1,& tx_remoto),
                }    
            }
            else {
                continue;
            }
        }
        }
    fn send_message(socket_send: & UdpSocket, camara:&mut VideoCapture, encoder:&mut EncoderRtc,tx_local: & Sender<Mat>) -> Result<(),ClientErrors>{
        let chunk_size: usize = 1200;
        // Leo frame y lo cargo al emisor local, la UI va a leerlos cuando la cámara esté habilitada
        let frame = video_processing::read_frame(camara).map_err(|_e| {
            ClientErrors::VideoProcessingError(VideoProcessingErrors::CamaraSettup)
        })?;
        let camara_width = video_processing::get_frame_width(&camara)?;
        let camara_height = video_processing::get_frame_height(&camara)?;
        let encoded_frame = match encoder.encode(&frame) {
            Ok(f) => f,
            Err(_e) => Vec::new()
        };

        if tx_local.send(frame).is_err() {
            println!("PROBLEMA");
        }
        if encoded_frame.is_empty() {return Ok(());};

        // Serializo el frame encodeado
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
        Ok(())
    }
    fn recieve_message(socket_recev: & UdpSocket) -> Option<Message> {
        let mut buff = vec![0u8; 60000];
        match socket_recev.recv(&mut buff) {
            Ok(0) => None,
            Ok(n) => { 
                match Message::deserializable(&buff[..n]) {
                    Some(msg) => Some(msg),
                    None => None
                } 
            },
            Err(_) => None
        }
    }

    fn signal_handle(signal:Signal, in_call: & Arc<AtomicBool>){
        match signal {
            Signal::EndCall => {
                // El otro cliente genera la señal, se deben terminar los threads de la llamada
                in_call.store(false, Ordering::SeqCst);
            }
    }
    }

    fn video_handle(video_chunk:Vec<u8>, decoder:&mut DecoderRtc,chunks_hash: &mut HashMap<u64, Vec<(u32, Vec<u8>)>>,width:i32,height:i32,tx_remoto: & Sender<Mat>) {
        let chunk_size = 1200;
        if let Some(rtp) = RtpPackage::from_bytes(&video_chunk) {
            let time_stamp = rtp.time_stamp;
            let entry = chunks_hash.entry(time_stamp).or_default();
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
                    width,
                    height,
                ) {
                    Ok(frame) => {
                        // println!("frame decoded");
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
            match decoder.decode(
                &video_chunk,
                width,
                height,
            ) {
                Ok(frame) => {
                    //println!("frame decoded");
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
