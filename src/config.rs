use serde::Deserialize;
use std::fs;

use crate::server_users::ServerUserData;

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct AppParameters {
    pub sdp_handshake: SdpHandshakeParameters,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct SdpHandshakeParameters {
    pub local_adder: String,
    pub server_adder: String,
    pub ice_candidates: Vec<String>,
    pub codecs: Vec<String>,
    pub offer_path: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct SignallingServerParameters {
    pub server_adder: String,
    pub users_db_path: String 
}

pub fn get_config_app(config_file_path: &str) -> Result<AppParameters, Box<dyn std::error::Error>> {
    let config = fs::read_to_string(config_file_path)?;
    let session_info: AppParameters = serde_json::from_str(&config)?;

    Ok(session_info)
}

pub fn get_config_signalling(
    config_file_path: &str,
) -> Result<SignallingServerParameters, Box<dyn std::error::Error>> {
    let config = fs::read_to_string(config_file_path)?;
    let server_info: SignallingServerParameters = serde_json::from_str(&config)?;

    Ok(server_info)
}

#[test]
fn test_config_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let config = fs::read_to_string("src/prueba_config.json")?;

    let session_info: AppParameters = serde_json::from_str(&config)?;

    let correct_sdp_info = SdpHandshakeParameters {
        local_adder: "192.168.0.109:8000".to_string(),
        server_adder: "192.168.0.109:8001".to_string(),
        ice_candidates: vec![
            "192.168.0.109:8002".to_string(),
            "192.168.0.109:8003".to_string(),
            "192.168.0.109:8004".to_string(),
        ],
        codecs: vec!["VP8".to_string(), "H264".to_string()],
        offer_path: "src/offer.sdp".to_string(),
    };

    let correct_app_info: AppParameters = AppParameters {
        sdp_handshake: correct_sdp_info,
    };

    assert_eq!(session_info, correct_app_info);
    Ok(())
}
