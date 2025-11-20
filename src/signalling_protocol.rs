use eframe::egui::TextBuffer;
use serde::{Deserialize, Serialize};

use crate::json_parser::json_clasify;

/// Mensajes que envía el cliente al servidor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    // #[serde(rename = "login")]
    Login { username: String, password: String },

    // #[serde(rename = "logout")]
    Logout { username: String },

    // #[serde(rename = "get_users")]
    GetUsers {},
}

/// Mensajes que envía el servidor al cliente
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(tag = "type")]
pub enum ServerMessage {
    // #[serde(rename = "login_ok")]
    LoginOk { username: String },

    // #[serde(rename = "login_error")]
    LoginError { reason: String },

    // #[serde(rename = "logout_ok")]
    LogoutOk,

    // #[serde(rename = "User_list")]
    UserList { users: String },

    // #[serde(rename = "error")]
    Error { reason: String },
}

impl ClientMessage {
    pub fn to_json_line(&self) -> String {
        // let mut json = serde_json::to_string(self)?;
        // json.push('\n');
        // Ok(json)

        match self {
            ClientMessage::Login { username, password } => format!(
                "{{\"type\":\"login\",\"username\":\"{}\",\"password\":\"{}\"}}\n",
                username, password
            ),

            ClientMessage::Logout { username } => {
                format!("{{\"type\":\"logout\",\"username\":\"{}\"}}\n", username)
            }

            ClientMessage::GetUsers {} => "{\"type\":\"get_users\"}\n".to_string(),
        }
    }

    pub fn from_json(json: &str) -> Option<Self> {
        let json_parsed = json_clasify(json);
        let parsed_objects = json_parsed.as_obj();

        let msg_type = parsed_objects.get("type")?.as_str();

        match msg_type.as_str() {
            "login" => Some(ClientMessage::Login {
                username: parsed_objects.get("username")?.as_str(),
                password: parsed_objects.get("password")?.as_str(),
            }),

            "logout" => Some(ClientMessage::Logout {
                username: parsed_objects.get("username")?.as_str(),
            }),

            "get_users" => Some(ClientMessage::GetUsers {}),

            _ => None,
        }
    }
}

impl ServerMessage {
    pub fn to_json_line(&self) -> String {
        // let mut json = serde_json::to_string(self)?;
        // json.push('\n');
        // Ok(json)

        match self {
            ServerMessage::LoginOk { username } => {
                format!("{{\"type\":\"login_ok\",\"username\":\"{}\"}}\n", username)
            }

            ServerMessage::LogoutOk => "{\"type\":\"logout_ok\"}\n".to_string(),

            ServerMessage::LoginError { reason } => {
                format!("{{\"type\":\"login_error\",\"reason\":\"{}\"}}\n", reason)
            }

            ServerMessage::UserList { users } => {
                format!("{{\"type\":\"user_list\",\"users\":\"{}\"}}\n", users)
            }

            ServerMessage::LoginError { reason } => {
                format!("{{\"type\":\"login_error\",\"reason\":\"{}\"}}\n", reason)
            }

            ServerMessage::Error { reason } => {
                format!("{{\"type\":\"error\",\"reason\":\"{}\"}}\n", reason)
            }
        }
    }

    pub fn from_json(json: &str) -> Option<Self> {
        let json_parsed = json_clasify(json);
        let parsed_objects = json_parsed.as_obj();

        let msg_type = parsed_objects.get("type")?.as_str();

        match msg_type.as_str() {
            "login_ok" => Some(ServerMessage::LoginOk {
                username: parsed_objects.get("username")?.as_str(),
            }),

            "login_error" => Some(ServerMessage::LoginError {
                reason: parsed_objects.get("reason")?.as_str(),
            }),

            "logout_ok" => Some(ServerMessage::LogoutOk),

            "User_list" => Some(ServerMessage::UserList {
                users: parsed_objects.get("users")?.as_str(),
            }),

            "error" => Some(ServerMessage::Error {
                reason: parsed_objects.get("reason")?.as_str(),
            }),

            _ => None,
        }
    }
}

fn replace_special_character(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub fn vec_to_json_array(vec_string: &Vec<String>) -> String {
    let mut output_str = String::from("[");
    for (i, str) in vec_string.iter().enumerate() {
        output_str.push('"');
        output_str.push_str(&replace_special_character(str));
        output_str.push('"');

        if (i + 1) < vec_string.len() {
            output_str.push(',');
        }
    }
    output_str.push(']');
    output_str
}
