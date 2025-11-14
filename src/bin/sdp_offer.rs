use std::sync::{Arc, atomic::AtomicBool};

use room_rtc::config;
use room_rtc::config::get_config_app;
use room_rtc::sdp_protocol::*;

fn main() {
    // cargo run --bin sdp_offer <ip:port> <ip:port>

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return;
    }

    // Se levanta cargo run --bin sdp_offer MI_IP:MI_PUERTO IP_PEER:PUERTO_PEER
    // en otro terminal se levanta cargo run --bin sdp_answer IP_PEER:PUERTO_PEER MI_IP:MI_PUERTO

    // Los ICE candidates están hardocodeados recordar cambiarlos de acuerdo a su IP local
    // Cambiar IP:PUERTO en líneas 90-93 de sdp_protocol.rs

    // Ejemplo: Attribute::Value("candidate".to_string(), "192.168.0.109 8000".to_string()),
    // Solamente cambiar "192.168.0.109 8000" el resto no.

    // RECORDAR: Si lo corren en dos compus cada compu cambia lo suyo
    // let ice_info = match sdp_handshake(Arc::new(AtomicBool::new(true)), &args[1], &args[2]) {
    //     Err(e) => {
    //         eprintln!("SDP Handshake error: {}", e);
    //         return;
    //     }
    //     Ok(info) => info,
    // };
    let session_info = match get_config_app(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprint!("Error config: {:?}", e);
            return;
        }
    };
    let _ = sdp_handshake(Arc::new(AtomicBool::new(true)), session_info.sdp_handshake);

    // La terminal que fue el lado que envió el Offer mostrará algo como: [DEBUG] LADO OFFER
    // La termina que fue el lado que recibió Offer y envió Answer mostrará algo como: [DEBUG] LADO ANSWER

    // println!("[DEBUG] ICE info local: {:?}\n", ice_info.0);
    // println!("[DEBUG] ICE info peer: {:?}\n", ice_info.1);
    // println!("[DEBUG] Codecs: {:?}\n", ice_info.2);

    // Se debe mostrar algo así:
    // [DEBUG] ICE info local: ["192.168.0.109:8000", "192.168.0.109:8001", "192.168.0.109:8002", "192.168.0.109:8080"]
    // [DEBUG] ICE info peer: ["192.168.0.109:8000"]
    // [DEBUG] Codecs: ["H264"]
}
