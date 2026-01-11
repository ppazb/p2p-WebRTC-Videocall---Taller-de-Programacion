use crate::json_parser::json_clasify;
use std::io::{self, ErrorKind, Write};

/// Mensajes que envía el cliente al servidor
#[derive(Debug, Clone)]
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

    CallStarted {
        username: String,
        target_user: String,
    },
}

/// Mensajes que envía el servidor al cliente
#[derive(Debug, Clone)]
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

    CalledUserBusy,

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

            ClientMessage::CallStarted {
                username,
                target_user,
            } => format!(
                "{{\"type\":\"call_started\",\"username\":\"{}\",\"target_user\":\"{}\"}}\n",
                replace_special_character(username),
                replace_special_character(target_user)
            ),

            ClientMessage::Offer {
                username,
                user_id,
                password,
                offer,
                target_user,
            } => {
                let target_field = match target_user {
                    Some(t) => format!("\"{}\"", replace_special_character(t)),
                    None => "null".to_string(),
                };
                format!(
                    "{{\"type\":\"offer\",\"username\":\"{}\",\"user_id\":\"{}\",\"password\":\"{}\",\"offer\":\"{}\",\"target_user\":{}}}\n",
                    replace_special_character(username),
                    replace_special_character(user_id),
                    replace_special_character(password),
                    replace_special_character(offer),
                    target_field
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
                "{{\"type\":\"call_end\",\"username\":\"{}\",\"target_user\":\"{}\"}}\n",
                replace_special_character(username),
                replace_special_character(target_user)
            ),
            ClientMessage::CallReject {
                username,
                target_user,
            } => format!(
                "{{\"type\":\"call_reject\",\"username\":\"{}\",\"target_user\":\"{}\"}}\n",
                replace_special_character(username),
                replace_special_character(target_user)
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
            "call_started" => Some(ClientMessage::CallStarted {
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
            ServerMessage::LoginOk { username } => format!(
                "{{\"type\":\"login_ok\",\"username\":\"{}\"}}\n",
                replace_special_character(username)
            ),

            ServerMessage::CalledUserBusy => "{\"type\":\"called_user_busy\"}\n".to_string(),

            ServerMessage::LogoutOk => "{\"type\":\"logout_ok\"}\n".to_string(),

            ServerMessage::LoginError { reason } => format!(
                "{{\"type\":\"login_error\",\"reason\":\"{}\"}}\n",
                replace_special_character(reason)
            ),

            ServerMessage::SignUpOk { username } => format!(
                "{{\"type\":\"signup_ok\",\"username\":\"{}\"}}\n",
                replace_special_character(username)
            ),

            ServerMessage::SignUpError { reason } => format!(
                "{{\"type\":\"signup_error\",\"reason\":\"{}\"}}\n",
                replace_special_character(reason)
            ),

            ServerMessage::UserList { users } => {
                let json_array = vec_to_json_array(users);
                format!("{{\"type\":\"user_list\",\"users\":{}}}\n", json_array)
            }

            ServerMessage::UserJoined { username } => format!(
                "{{\"type\":\"user_joined\",\"username\":\"{}\"}}\n",
                replace_special_character(username)
            ),

            ServerMessage::UserLeft { username } => format!(
                "{{\"type\":\"user_left\",\"username\":\"{}\"}}\n",
                replace_special_character(username)
            ),

            ServerMessage::OfferForwarded { from_user, offer } => format!(
                "{{\"type\":\"offer_forwarded\",\"from_user\":\"{}\",\"offer\":\"{}\"}}\n",
                replace_special_character(from_user),
                replace_special_character(offer)
            ),

            ServerMessage::OfferAvailable { from_user, offer } => format!(
                "{{\"type\":\"offer_available\",\"from_user\":\"{}\",\"offer\":\"{}\"}}\n",
                replace_special_character(from_user),
                replace_special_character(offer)
            ),

            ServerMessage::AnswerForwarded { from_user, answer } => format!(
                "{{\"type\":\"answer_forwarded\",\"from_user\":\"{}\",\"answer\":\"{}\"}}\n",
                replace_special_character(from_user),
                replace_special_character(answer)
            ),

            ServerMessage::CallEnded { from_user } => format!(
                "{{\"type\":\"call_ended\",\"from_user\":\"{}\"}}\n",
                replace_special_character(from_user)
            ),

            ServerMessage::CallRejected { from_user } => format!(
                "{{\"type\":\"call_rejected\",\"from_user\":\"{}\"}}\n",
                replace_special_character(from_user)
            ),

            ServerMessage::Error { reason } => format!(
                "{{\"type\":\"error\",\"reason\":\"{}\"}}\n",
                replace_special_character(reason)
            ),
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

            "called_user_busy" => Some(ServerMessage::CalledUserBusy),

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
                let arr = parsed_objects.get("users")?.as_str_array();
                Some(ServerMessage::UserList { users: arr })
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

pub fn vec_to_json_array(vec_string: &[String]) -> String {
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

pub fn write_all_nonblocking<W: Write>(writer: &mut W, mut data: &[u8]) -> io::Result<()> {
    while !data.is_empty() {
        match writer.write(data) {
            Ok(0) => {
                return Err(io::Error::new(
                    ErrorKind::WriteZero,
                    "no se pudo escribir en el stream",
                ));
            }
            Ok(n) => data = &data[n..], // avanzamos los bytes ya enviados
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    writer.flush()
}
