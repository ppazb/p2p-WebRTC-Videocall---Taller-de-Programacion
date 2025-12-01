
use std::{collections::HashMap, fs};

use crate::json_parser::*;

#[derive(Debug, PartialEq, Clone)]
pub struct AppParameters {
    pub general_info: GeneralAppInfo,
    pub sdp_handshake: SdpHandshakeParameters,
}

impl AppParameters {
    fn from_json(obj: &HashMap<String, JsonStruct>) -> Self {
        AppParameters {
            general_info: GeneralAppInfo::from_json(&obj["general_info"].as_obj()),
            sdp_handshake: SdpHandshakeParameters::from_json(&obj["sdp_handshake"].as_obj()),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GeneralAppInfo {
    pub server_adder: String,
}

impl GeneralAppInfo {
    fn from_json(obj: &HashMap<String, JsonStruct>) -> Self {
        GeneralAppInfo {
            server_adder: obj["server_adder"].as_str(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SdpHandshakeParameters {
    pub local_adder: String,
    pub server_adder: String,
    pub ice_candidates: Vec<String>,
    pub codecs: Vec<String>,
    pub offer_path: String,
}

impl SdpHandshakeParameters {
    fn from_json(obj: &HashMap<String, JsonStruct>) -> Self {
        SdpHandshakeParameters {
            local_adder: obj["local_adder"].as_str(),
            server_adder: obj["server_adder"].as_str(),
            codecs: obj["codecs"].as_str_array(),
            ice_candidates: obj["ice_candidates"].as_str_array(),
            offer_path: obj["offer_path"].as_str(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SignallingServerParameters {
    pub server_adder: String,
    pub users_db: String,
}

impl SignallingServerParameters {
    fn from_json(obj: &HashMap<String, JsonStruct>) -> Self {
        println!("keys = {:?}", obj.keys());
        SignallingServerParameters {
            server_adder: obj["server_adder"].as_str(),
            users_db: obj["users_db"].as_str(),
        }
    }
}

pub fn get_config_app(config_file_path: &str) -> Result<AppParameters, Box<dyn std::error::Error>> {
    let config = fs::read_to_string(config_file_path)?;
    println!("\n[DEBUG] READ TO STRING de un archivo JSON: {:?}", config);
    let config_clasification = json_clasify(&config);
    let config_objects = config_clasification.as_obj();
    let app_params_struct = AppParameters::from_json(&config_objects);
    println!("\n\n[DEBUG] APP CONFIG: {:?}", app_params_struct);

    Ok(app_params_struct)
}

pub fn get_config_signalling(
    config_file_path: &str,
) -> Result<SignallingServerParameters, Box<dyn std::error::Error>> {
    let config = fs::read_to_string(config_file_path)?;
    let config_clasification = json_clasify(&config);
    let config_objects = config_clasification.as_obj();
    println!("\n\n[DEBUG] CONFIG OBJETS: {:?}", &config_objects);
    let server_info: SignallingServerParameters =
        SignallingServerParameters::from_json(&config_objects);
    println!("\n\n[DEBUG] SERVER CONFIG: {:?}", server_info);

    Ok(server_info)
}

// #[test]
// fn test_config_serialization() -> Result<(), Box<dyn std::error::Error>> {
//     let config = fs::read_to_string("src/prueba_config.json")?;

//     let session_info: AppParameters = serde_json::from_str(&config)?;

//     let correct_sdp_info = SdpHandshakeParameters {
//         local_adder: "192.168.0.109:8000".to_string(),
//         server_adder: "192.168.0.109:8001".to_string(),
//         ice_candidates: vec![
//             "192.168.0.109:8002".to_string(),
//             "192.168.0.109:8003".to_string(),
//             "192.168.0.109:8004".to_string(),
//         ],
//         codecs: vec!["VP8".to_string(), "H264".to_string()],
//         offer_path: "src/offer.sdp".to_string(),
//     };

//     let correct_app_info: AppParameters = AppParameters {
//         sdp_handshake: correct_sdp_info,
//     };

//     assert_eq!(session_info, correct_app_info);
//     Ok(())
// }
