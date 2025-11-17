use serde::{Deserialize, Serialize};

/// Mensajes que envía el cliente al servidor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "login")]
    Login { username: String, password: String },
    
    #[serde(rename = "logout")]
    Logout { username: String },
    
    #[serde(rename = "get_users")]
    GetUsers { },
}

/// Mensajes que envía el servidor al cliente
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "login_ok")]
    LoginOk { username: String },
    
    #[serde(rename = "login_error")]
    LoginError { reason: String },
    
    #[serde(rename = "logout_ok")]
    LogoutOk,

    #[serde(rename = "User_list")]
    UserList { users: String }, 
    
    #[serde(rename = "error")]
    Error { reason: String },
}

impl ClientMessage {
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}

impl ServerMessage {
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}
