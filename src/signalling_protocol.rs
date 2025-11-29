use serde::{Deserialize, Serialize};

use crate::json_parser::json_clasify;

/// Mensajes que envía el cliente al servidor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Login {
        username: String,
        password: String,
    },

    SignUp {
        username: String,
        password: String,
    },

    Logout {
        username: String,
    },

    GetUsers {},

    Offer {
        username: String,
        user_id: String,
        password: String,
        offer: String,
        target_user: Option<String>,
    },

    Answer {
        username: String,
        user_id: String,
        answer: String,
        target_user: String,
    },

    RequestOffer {
        username: String,
        target_user: String,
    },

    CallEnd {
        username: String,
        target_user: String,
    },

    CallReject {
        username: String,
        target_user: String,
    },
}

/// Mensajes que envía el servidor al cliente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    LoginOk { username: String },

    LoginError { reason: String },

    SignUpOk { username: String },

    SignUpError { reason: String },

    LogoutOk,

    UserList { users: Vec<String> },

    UserJoined { username: String },

    UserLeft { username: String },

    OfferForwarded { from_user: String, offer: String },

    OfferAvailable { from_user: String, offer: String },

    AnswerForwarded { from_user: String, answer: String },

    CallEnded { from_user: String },
    CallRejected { from_user: String },

    Error { reason: String },
}

impl ClientMessage {
    pub fn to_json_line(&self) -> String {
        match self {
            ClientMessage::Login { username, password } => format!(
                "{{\"type\":\"login\",\"username\":\"{}\",\"password\":\"{}\"}}\n",
                username, password
            ),

            ClientMessage::SignUp { username, password } => format!(
                "{{\"type\":\"signup\",\"username\":\"{}\",\"password\":\"{}\"}}\n",
                username, password
            ),

            ClientMessage::Logout { username } => {
                format!("{{\"type\":\"logout\",\"username\":\"{}\"}}\n", username)
            }

            ClientMessage::GetUsers {} => "{\"type\":\"get_users\"}\n".to_string(),

            ClientMessage::Offer {
                username,
                user_id,
                password,
                offer,
                target_user,
            } => {
                let target = match target_user {
                    Some(t) => t.clone(),
                    None => "".to_string(),
                };
                format!(
                    "{{\"type\":\"offer\",\"username\":\"{}\",\"user_id\":\"{}\",\"password\":\"{}\",\"offer\":\"{}\",\"target_user\":\"{}\"}}\n",
                    replace_special_character(username),
                    replace_special_character(user_id),
                    replace_special_character(password),
                    replace_special_character(offer),
                    replace_special_character(&target)
                )
            }

            ClientMessage::Answer {
                username,
                user_id,
                answer,
                target_user,
            } => format!(
                "{{\"type\":\"answer\",\"username\":\"{}\",\"user_id\":\"{}\",\"answer\":\"{}\",\"target_user\":\"{}\"}}\n",
                replace_special_character(username),
                replace_special_character(user_id),
                replace_special_character(answer),
                replace_special_character(target_user)
            ),

            ClientMessage::RequestOffer {
                username,
                target_user,
            } => format!(
                "{{\"type\":\"request_offer\",\"username\":\"{}\",\"target_user\":\"{}\"}}\n",
                replace_special_character(username),
                replace_special_character(target_user)
            ),

            ClientMessage::CallEnd {
                username,
                target_user,
            } => format!(
                "{{\"type\":\"{}\",\"end_call\":\"\",\"target_user\":\"{}\"}}\n",
                username, target_user
            ),
            ClientMessage::CallReject {
                username,
                target_user,
            } => format!(
                "{{\"type\":\"call_reject\",\"username\":\"{}\",\"target_user\":\"{}\"}}\n",
                username, target_user
            ),
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

            "signup" => Some(ClientMessage::SignUp { 
                username: parsed_objects.get("username")?.as_str(), 
                password: parsed_objects.get("password")?.as_str(), 
            }),

            "logout" => Some(ClientMessage::Logout {
                username: parsed_objects.get("username")?.as_str(),
            }),

            "get_users" => Some(ClientMessage::GetUsers {}),

            "offer" => Some(ClientMessage::Offer {
                username: parsed_objects.get("username")?.as_str(),
                user_id: parsed_objects.get("user_id")?.as_str(),
                password: parsed_objects.get("password")?.as_str(),
                offer: parsed_objects.get("offer")?.as_str(),
                target_user: parsed_objects.get("target_user").map(|v| v.as_str()),
            }),

            "answer" => Some(ClientMessage::Answer {
                username: parsed_objects.get("username")?.as_str(),
                user_id: parsed_objects.get("user_id")?.as_str(),
                answer: parsed_objects.get("answer")?.as_str(),
                target_user: parsed_objects.get("target_user")?.as_str(),
            }),

            "request_offer" => Some(ClientMessage::RequestOffer {
                username: parsed_objects.get("username")?.as_str(),
                target_user: parsed_objects.get("target_user")?.as_str(),
            }),
            "call_end" => Some(ClientMessage::CallEnd {
                username: parsed_objects.get("username")?.as_str(),
                target_user: parsed_objects.get("target_user")?.as_str(),
            }),
            "call_reject" => Some(ClientMessage::CallReject {
                username: parsed_objects.get("username")?.as_str(),
                target_user: parsed_objects.get("target_user")?.as_str(),
            }),
            _ => None,
        }
    }
}

impl ServerMessage {
    pub fn to_json_line(&self) -> String {
        match self {
            ServerMessage::LoginOk { username } => {
                format!(
                    "{{\"type\":\"login_ok\",\"username\":\"{}\"}}\n",
                    replace_special_character(username)
                )
            }

            ServerMessage::LogoutOk => "{\"type\":\"logout_ok\"}\n".to_string(),

            ServerMessage::LoginError { reason } => {
                format!(
                    "{{\"type\":\"login_error\",\"reason\":\"{}\"}}\n",
                    replace_special_character(reason)
                )
            }

            ServerMessage::SignUpOk { username} => {
                format!(
                    "{{\"type\":\"signup_ok\",\"username\":\"{}\"}}\n",
                    replace_special_character(username)
                )
            }

            ServerMessage::SignUpError { reason } => {
                format!(
                    "{{\"type\":\"signup_error\",\"reason\":\"{}\"}}\n",
                    replace_special_character(reason)
                )
            }

            ServerMessage::UserList { users } => {
                let users_json_array = vec_to_json_array(users);

                let users_escaped = users_json_array.replace("\"", "\\\"");

                format!(
                    "{{\"type\":\"user_list\",\"users\":\"{}\"}}\n",
                    users_escaped
                )
            }

            ServerMessage::UserJoined { username } => {
                format!(
                    "{{\"type\":\"user_joined\",\"username\":\"{}\"}}\n",
                    replace_special_character(username)
                )
            }

            ServerMessage::UserLeft { username } => {
                format!(
                    "{{\"type\":\"user_left\",\"username\":\"{}\"}}\n",
                    replace_special_character(username)
                )
            }

            ServerMessage::OfferForwarded { from_user, offer } => {
                format!(
                    "{{\"type\":\"offer_forwarded\",\"from_user\":\"{}\",\"offer\":\"{}\"}}\n",
                    replace_special_character(from_user),
                    replace_special_character(offer)
                )
            }

            ServerMessage::OfferAvailable { from_user, offer } => {
                format!(
                    "{{\"type\":\"offer_available\",\"from_user\":\"{}\",\"offer\":\"{}\"}}\n",
                    replace_special_character(from_user),
                    replace_special_character(offer)
                )
            }

            ServerMessage::AnswerForwarded { from_user, answer } => {
                format!(
                    "{{\"type\":\"answer_forwarded\",\"from_user\":\"{}\",\"answer\":\"{}\"}}\n",
                    replace_special_character(from_user),
                    replace_special_character(answer)
                )
            }
            ServerMessage::CallEnded { from_user } => {
                format!(
                    "{{\"type\":\"call_end\",\"from_user\":\"{}\",\"and_user\":\"\"}}\n",
                    from_user
                )
            }

            ServerMessage::CallRejected { from_user } => {
                format!(
                    "{{\"type\":\"call_rejected\",\"from_user\":\"{}\"}}\n",
                    from_user
                )
            }

            ServerMessage::Error { reason } => {
                format!(
                    "{{\"type\":\"error\",\"reason\":\"{}\"}}\n",
                    replace_special_character(reason)
                )
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

            "signup_ok" => Some(ServerMessage::SignUpOk {
                username: parsed_objects.get("username")?.as_str(),
            }),

            "signup_error" => Some(ServerMessage::SignUpError {
                reason: parsed_objects.get("reason")?.as_str(),
            }),

            "user_list" | "User_list" => {
                // AHORA SÍ: .as_str() funcionará porque enviamos un string
                let raw_users = parsed_objects.get("users")?.as_str();
                Some(ServerMessage::UserList {
                    users: parse_string_array(&raw_users),
                })
            }

            "user_joined" => Some(ServerMessage::UserJoined {
                username: parsed_objects.get("username")?.as_str(),
            }),

            "user_left" => Some(ServerMessage::UserLeft {
                username: parsed_objects.get("username")?.as_str(),
            }),

            "offer_forwarded" => Some(ServerMessage::OfferForwarded {
                from_user: parsed_objects.get("from_user")?.as_str(),
                offer: parsed_objects.get("offer")?.as_str(),
            }),

            "offer_available" => Some(ServerMessage::OfferAvailable {
                from_user: parsed_objects.get("from_user")?.as_str(),
                offer: parsed_objects.get("offer")?.as_str(),
            }),

            "answer_forwarded" => Some(ServerMessage::AnswerForwarded {
                from_user: parsed_objects.get("from_user")?.as_str(),
                answer: parsed_objects.get("answer")?.as_str(),
            }),

            "call_ended" => Some(ServerMessage::CallEnded {
                from_user: parsed_objects.get("from_user")?.as_str(),
            }),

            "call_rejected" => Some(ServerMessage::CallRejected {
                from_user: parsed_objects.get("from_user")?.as_str(),
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

fn parse_string_array(json_array: &str) -> Vec<String> {
    let trimmed = json_array.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let content = &trimmed[1..trimmed.len() - 1];
        if content.is_empty() {
            return Vec::new();
        }
        content
            .split(',')
            .map(|s| s.trim().trim_matches('"').replace("\\\"", "\"").to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}
