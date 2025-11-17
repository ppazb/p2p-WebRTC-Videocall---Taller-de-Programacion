use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener};
use std::sync::{Arc, Mutex};
use room_rtc::server_users::ServerUserData;
use room_rtc::signalling_protocol::{ClientMessage, ServerMessage};
use room_rtc::config::*;
use room_rtc::signalling_error::SignallingServerError;

fn main() -> Result<(), SignallingServerError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <config_file>", args[0]);
        return Err(SignallingServerError::ConnectionError);
    }

    let server_info: SignallingServerParameters = match get_config_signalling(&args[1]) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("\n[ERROR] Config Error: {:?}", e);
            return Err(SignallingServerError::ConfigInfoError);
        }
    };
    println!("[SERVER] Starting on {}", server_info.server_adder);

    let listener = TcpListener::bind(&server_info.server_adder)
        .map_err(|_| SignallingServerError::ConnectionError)?;

    let file_path = &server_info.users_db;
    let login = ServerUserData::new(file_path).unwrap();
    let active_clients: Arc<Mutex<ServerUserData>> = Arc::new(Mutex::new(login));

    println!("[SERVER] Listening for connections...\n");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                println!("[SERVER] New connection received");
                let clients = Arc::clone(&active_clients);
                std::thread::spawn(move || {
                    let _ = handle_client(s, clients);
                });
            }
            Err(e) => {
                eprintln!("[ERROR] Connection error: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(
    stream: std::net::TcpStream,
    active_clients: Arc<Mutex<ServerUserData>>,
) -> std::io::Result<()> {
    let read_stream = stream.try_clone()?;
    let mut reader = BufReader::new(read_stream);
    let mut stream_write = stream;
    let mut current_user_id: Option<String> = None;

    // Loop infinito: leer múltiples mensajes del mismo cliente
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("[SERVER] Client disconnected");
                break;
            }
            Ok(_) => {
                eprintln!("[DEBUG SERVER] Received: {}", line);

                // Parsear JSON
                match serde_json::from_str::<ClientMessage>(&line) {
                    Ok(ClientMessage::Login { username, password  }) => {
                        {
                            let mut clients = active_clients.lock().unwrap();
                            if let Ok(result_verification) = clients.verificar_login(&username, &password) {
                                // Verificar que el cliente esté en estado desconectado (cambiar en server_user verficar_login)
                                clients.conectar(&username); 
                                eprintln!("[SERVER] User '{}' logged in", username);

                                let response = ServerMessage::LoginOk { username };
                                let response_json = response.to_json_line().unwrap();
                                stream_write.write_all(response_json.as_bytes())?;
                                stream_write.flush()?;
                                eprintln!("[SERVER] Sent LoginOk\n");  
                            } else {
                                let response = ServerMessage::LoginError { reason: "Datos invalidos".to_string() };
                                let response_json = response.to_json_line().unwrap();
                                stream_write.write_all(response_json.as_bytes())?;
                                stream_write.flush()?;
                                eprintln!("[SERVER] user or password invalid '{}' logged in", username);

                            }
                        }
                    }
                    Ok(ClientMessage::Logout { username }) => {
                        {
                            let mut clients = active_clients.lock().unwrap();
                            let _ = clients.desconectar(&username); 
                        }

                        eprintln!("[SERVER] User {} logged out", &username);
                        let response = ServerMessage::LogoutOk;
                        let response_json = response.to_json_line().unwrap();
                        stream_write.write_all(response_json.as_bytes())?;
                        stream_write.flush()?;
                        eprintln!("[SERVER] Sent LogoutOk\n");
                    }
                    Ok(ClientMessage::GetUsers {}) => {
                        
                        let clients_guard = active_clients.lock().unwrap();
                        let clients_map = clients_guard.get_listado_usuarios();
                        drop(clients_guard);

                        let users_list = format!(
                            "{{\"type\":\"users_list\",\"users\":{}}}\n",
                            serde_json::to_string(&clients_map).unwrap()
                        );
                        let response = ServerMessage::UserList { users: users_list };
                        let response_json = response.to_json_line().unwrap();
                        stream_write.write_all(response_json.as_bytes())?;
                        stream_write.flush()?;
                        eprintln!("[SERVER] Sent users list\n");
                    }
                    Err(e) => {
                        eprintln!("[ERROR] Failed to parse JSON: {}", e);
                        let response = ServerMessage::Error { reason: "Invalid JSON".to_string() };
                        let response_json = response.to_json_line().unwrap();
                        let _ = stream_write.write_all(response_json.as_bytes());
                    }
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Read error: {}", e);
                break;
            }
        }
    }


    Ok(())
}
