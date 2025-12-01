use room_rtc::config::*;
use room_rtc::server_users::ServerUserData;
use room_rtc::signalling_error::SignallingServerError;
use room_rtc::signalling_protocol::{ClientMessage, ServerMessage, write_all_nonblocking};
use room_rtc::tls_interface::{server_tls_config, server_try_tls_handshake};
use rustls::{ServerConnection, StreamOwned};
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use room_rtc::log_s;


#[derive(Debug, Clone)]
struct StoredOffer {
    from_user: String,
    offer: String,
}

fn main() -> Result<(), SignallingServerError> {
    room_rtc::log::setup_panic_hook();
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
    log_s!("\n\n[SERVER] Starting on {}", server_info.server_adder);

    let server_config = match server_tls_config() {
        Ok(arc_server_config) => arc_server_config,
        Err(e) => return Err(SignallingServerError::TlsHandshakeError(e)),
    };

    println!(
        "\n\n[SERVER] Listener Binding: {:?}",
        &server_info.server_adder
    );

    let listener = TcpListener::bind(&server_info.server_adder)
        .map_err(|_| SignallingServerError::ConnectionError)?;

    let file_path = &server_info.users_db;
    let login = ServerUserData::new(file_path).map_err(|e| SignallingServerError::FileNotFound(e))?;

    let active_clients: Arc<Mutex<ServerUserData>> = Arc::new(Mutex::new(login));

    let connected_clients: Arc<
        Mutex<HashMap<String, Arc<Mutex<StreamOwned<ServerConnection, TcpStream>>>>>,
    > = Arc::new(Mutex::new(HashMap::new()));

    let busy_clients: Arc<
        Mutex<HashMap<String, Arc<Mutex<StreamOwned<ServerConnection, TcpStream>>>>>,
    > = Arc::new(Mutex::new(HashMap::new()));

    // persistir ofertas
    let stored_offers: Arc<Mutex<HashMap<String, StoredOffer>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    println!("[SERVER] Listening for connections...\n");

    for stream in listener.incoming() {
        let server_config_each_client = Arc::clone(&server_config);
        match stream {
            Ok(s) => {
                println!("\n\n[SERVER] New connection received");
                log_s!("\n\n[SERVER] New connection received");
                let clients = Arc::clone(&active_clients);
                let connections = Arc::clone(&connected_clients);
                let busy_clients = Arc::clone(&busy_clients);
                let offers = Arc::clone(&stored_offers);

                let tls_stream = match server_try_tls_handshake(server_config_each_client, s) {
                    Ok(tls_s) => {
                        println!("\n\n[SERVER] TLS Handshake Success");
                        tls_s
                    }
                    Err(e) => return Err(SignallingServerError::TlsHandshakeError(e)),
                };

                let client_handle = std::thread::spawn(move || {
                    let _ = handle_client(tls_stream, clients, connections, busy_clients, offers);
                });
                handles.push(client_handle);
            }
            Err(e) => {
                log_s!("[ERROR] Connection error: {}", e);
            }
        }
    }

    for handle in handles {
        let _ = handle.join();
    }
    room_rtc::log::flush_server();

    Ok(())
}

fn handle_client(
    stream: StreamOwned<ServerConnection, TcpStream>,
    active_clients: Arc<Mutex<ServerUserData>>,
    connected_clients: Arc<
        Mutex<HashMap<String, Arc<Mutex<StreamOwned<ServerConnection, TcpStream>>>>>,
    >,
    busy_clients: Arc<Mutex<HashMap<String, Arc<Mutex<StreamOwned<ServerConnection, TcpStream>>>>>>,
    stored_offers: Arc<Mutex<HashMap<String, StoredOffer>>>,
) -> std::io::Result<()> {
    stream.get_ref().set_nonblocking(true)?;
    let tls_stream = Arc::new(Mutex::new(stream));
    let read_tls = tls_stream.clone();
    let write_tls = tls_stream.clone();

    let mut current_user: Option<String> = None;

    // Buffer acumulativo para reconstruir mensajes fragmentados
    let mut pending_buffer: Vec<u8> = Vec::new();

    let mut buffer = [0u8; 4096];
    
    // Loop infinito: leer múltiples mensajes del mismo cliente
    loop {
        let mut guard_read = match read_tls.lock().map_err(|_e| SignallingServerError::SyncError) {
            Ok(rg) => rg,
            Err(e) => {
                return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
            }
        };

        let n = match guard_read.read(&mut buffer) {
            // Si da 0 es porque se cierra el socket -> se que se desconecta el usuario

            Ok(0) => 0, 
            Ok(n) => n,
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                drop(guard_read);
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            Err(e) => {
                eprintln!("[ERROR] Read error: {}", e);
                break;
            }
        };

        drop(guard_read);

        if n == 0 {
            log_s!("[SERVER] Client disconnected (socket closed)");

            if let Some(user) = &current_user {
                {
                    // Dejar los cochetes es para que se dropee el lock cuando salga del scope
                    // Sacar del mapa de conexiones
                    let mut clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: Cant use resource now {e}")))?;
                    clients_map.remove(user);

                    //de haber estado ocupado sacar tambien de ahi
                    let mut busy_map = busy_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: Cant use resource now {e}")))?;
                    busy_map.remove(user);

                    // Avisar a todos los demás que este usuario se fue
                    let msg_left = ServerMessage::UserLeft {
                        username: user.clone(),
                    };
                    let msg_json = msg_left.to_json_line();

                    for (_, other_stream) in clients_map.iter_mut() {
                        match other_stream.lock() {
                            Ok(mut tls_s) => {
                                let _ = write_all_nonblocking(&mut *tls_s, msg_json.as_bytes());
                                tls_s.flush()?;
                            }
                            
                            Err(e) => {
                                return Err(std::io::Error::other(format!(
                                    "TLS mutex poisoned: {e}"
                                )));
                            }
                        };
                    }
                    log_s!("[SERVER] Removed {} and notified others", user);
                }
                // Desconectar el client
                let mut clients = active_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: Cant use resource now {e}")))?;
                let _ = clients.desconectar(user);
            }
            break;
        }

        pending_buffer.extend_from_slice(&buffer[..n]);

        while let Some(pos) = pending_buffer.iter().position(|&x| x == b'\n') {
            let line_bytes: Vec<u8> = pending_buffer.drain(..=pos).collect();
            
            if let Ok(line) = String::from_utf8(line_bytes) {
                println!("[SERVER] Line Read: {:?}", line.trim());

                match ClientMessage::from_json(&line) {
                    Some(ClientMessage::Login { username, password }) => {
                        let mut clients = active_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: Cant use resource now {e}")))?;
                        if clients.verificar_login(&username, &password).is_ok() {
                            // Marcar como conectado en la BD
                            let _ = clients.conectar(&username);
                            let clients_list: Vec<String>;
                            log_s!("[SERVER] User '{}' logged in", username);
                            let msg_joined = ServerMessage::UserJoined {
                                username: username.clone(),
                            };
                            let msg_json = msg_joined.to_json_line();

                            //scope para evitar lock
                            {
                                let mut clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: Cant use resource now {e}")))?;

                                for (other_user, other_stream) in clients_map.iter_mut() {
                                    let _tls_other_stream = match other_stream.lock() {
                                        Ok(mut tls_s) => {
                                            if other_user != &username {
                                                let _ = write_all_nonblocking(&mut *tls_s, msg_json.as_bytes());
                                                let _ = tls_s.flush();
                                            }
                                            tls_s
                                        }
                                        Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                                    };
                                }

                                // Guardar la conexión de este usuario
                                // if let Ok(write_clone) = stream_write.try_clone() {
                                clients_map.insert(username.clone(), write_tls.clone());
                                clients_list = clients_map.keys().cloned().collect();
                            }

                            {
                                let mut tls_write = match write_tls.lock() {
                                    Ok(s) => s,
                                    Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                                };
                                println!("\n\n[SERVER] Login Success");
                                let response = ServerMessage::LoginOk {
                                    username: username.clone(),
                                };
                                let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                                let _ = tls_write.flush();
                                log_s!("[SERVER] Sent LoginOk\n");
                                // Le enviamos la lista una vez que entra, ya que la primera vez
                                // el usuario todavia no estaba adentro

                                let user_list = ServerMessage::UserList {
                                    users: clients_list,
                                };
                                let list_json = user_list.to_json_line();
                                let _ = write_all_nonblocking(&mut *tls_write, list_json.as_bytes());
                                let _ = tls_write.flush();
                                log_s!("[SERVER] Sent Initial UserList to new user");
                            }
                            current_user = Some(username.clone());
                        } else {
                            let response = ServerMessage::LoginError {
                                reason: "Datos invalidos o usuario ya conectado".to_string(),
                            };
                            let mut tls_write = match write_tls.lock() {
                                Ok(s) => s,
                                Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                            };
                            let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                            let _ = tls_write.flush();
                            log_s!("[SERVER] Login failed for '{}'", username);
                        }
                    }
                    Some(ClientMessage::CallStarted { username, target_user }) => {
                        log_s!("[SERVER] CallStarted from {} to {}", username, target_user);

                        // checkeo si alguno de los dos ya está ocupado
                        let mut busy_map = busy_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: Cant use resource now {e}")))?;
                        if busy_map.contains_key(&target_user) || busy_map.contains_key(&username) {
                            let response = ServerMessage::CalledUserBusy;
                            let mut tls_write = match write_tls.lock() {
                                Ok(s) => s,
                                Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                            };
                            let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                            tls_write.flush()?;
                            continue;
                        }
                        let clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        if let (Some(user1), Some(user2)) = (clients_map.get(&username), clients_map.get(&target_user)) {
                            busy_map.insert(username.clone(), user1.clone());
                            busy_map.insert(target_user.clone(), user2.clone());
                            log_s!("[SERVER] Marked {} and {} as busy", username, target_user);
                        }
                    }
                    Some(ClientMessage::SignUp { username, password }) => {
                        let mut clients = active_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        match clients.verificar_signup(&username, &password) {
                            Ok(()) => {
                                // Agrego el usuario al archivo json
                                let _ = clients.add_user(&username, &password);
                                let _ = clients.conectar(&username);
                                let clients_list: Vec<String>;
                                {
                                    let mut clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                                    let msg_joined = ServerMessage::UserJoined { username: username.clone() };
                                    let msg_json = msg_joined.to_json_line();
                                    for (other_user, other_stream) in clients_map.iter_mut() {
                                        if let Ok(mut tls_s) = other_stream.lock() {
                                            if other_user != &username {
                                                let _ = write_all_nonblocking(&mut *tls_s, msg_json.as_bytes());
                                                let _ = tls_s.flush();
                                            }
                                        }
                                    }
                                    clients_map.insert(username.clone(), write_tls.clone());
                                    clients_list = clients_map.keys().cloned().collect();
                                }
                                current_user = Some(username.clone());
                                let mut tls_write = match write_tls.lock() {
                                    Ok(s) => s,
                                    Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                                };
                                let response = ServerMessage::LoginOk { username: username.clone() };
                                let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                                let user_list = ServerMessage::UserList { users: clients_list };
                                let _ = write_all_nonblocking(&mut *tls_write, user_list.to_json_line().as_bytes());
                                tls_write.flush()?;
                            }
                            Err(e) => {
                                let response = ServerMessage::SignUpError { reason: e.to_string() };
                                let mut tls_write = match write_tls.lock() {
                                    Ok(s) => s,
                                    Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                                };
                                 // Le enviamos la lista una vez que entra, ya que la primera vez
                                // el usuario todavia no estaba adentro
                                let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                                tls_write.flush()?;
                            }
                        }
                    }
                    Some(ClientMessage::Logout { username }) => {
                        let mut clients = active_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        let _ = clients.desconectar(&username);
                        let response = ServerMessage::LogoutOk;
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                        };
                        let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                        tls_write.flush()?;
                        
                        {
                            let mut clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                            clients_map.remove(&username);
                            let mut busy_map = busy_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                            busy_map.remove(&username);
                            
                            let msg_left = ServerMessage::UserLeft { username: username.clone() };
                            let msg_json = msg_left.to_json_line();
                            for (_, other_stream) in clients_map.iter_mut() {
                                if let Ok(mut s) = other_stream.lock() {
                                    let _ = write_all_nonblocking(&mut *s, msg_json.as_bytes());
                                    let _ = s.flush();
                                }
                            }
                        }
                        //La aplicacion en este socket no tiene usuario
                        //No cierro socket para agilizar si se vuelve a conectar -> consultar
                        current_user = None;
                    }
                    Some(ClientMessage::GetUsers {}) => {
                        let clients_guard = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        let users_vec: Vec<String> = clients_guard.keys().cloned().collect();
                        drop(clients_guard);
                        let response = ServerMessage::UserList { users: users_vec };
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                        };
                        let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                        tls_write.flush()?;
                    }
                    Some(ClientMessage::Offer { username, offer, target_user, .. }) => {
                        {
                            let mut offers_map = stored_offers.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                            offers_map.insert(username.clone(), StoredOffer { from_user: username.clone(), offer: offer.clone() });
                        }
                        if let Some(target) = target_user {
                            let busy_map = busy_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                            if busy_map.contains_key(&target) {
                                let response = ServerMessage::CalledUserBusy;
                                let mut tls_write = match write_tls.lock() {
                                    Ok(s) => s,
                                    Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                                };
                                let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                                tls_write.flush()?;
                            } else {
                                let mut clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                                if let Some(target_stream) = clients_map.get_mut(&target) {
                                    let response = ServerMessage::OfferForwarded { from_user: username.clone(), offer: offer.clone() };
                                    if let Ok(mut tls_s) = target_stream.lock() {
                                        let _ = write_all_nonblocking(&mut *tls_s, response.to_json_line().as_bytes());
                                        let _ = tls_s.flush();
                                    }
                                }
                            }
                        }
                    }
                    Some(ClientMessage::Answer { username, answer, target_user, .. }) => {
                        let mut clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        if let Some(target_stream) = clients_map.get_mut(&target_user) {
                            let response = ServerMessage::AnswerForwarded { from_user: username.clone(), answer: answer.clone() };
                            if let Ok(mut tls_s) = target_stream.lock() {
                                let _ = write_all_nonblocking(&mut *tls_s, response.to_json_line().as_bytes());
                                let _ = tls_s.flush();
                            }
                        }
                    }
                    Some(ClientMessage::RequestOffer { target_user, .. }) => {
                        let offers_map = stored_offers.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                        };
                        if let Some(stored) = offers_map.get(&target_user) {
                            let response = ServerMessage::OfferAvailable { from_user: stored.from_user.clone(), offer: stored.offer.clone() };
                            let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                            tls_write.flush()?;
                        } else {
                            let response = ServerMessage::Error { reason: format!("No offer available from {}", target_user) };
                            let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                        }
                    }
                     //aviso al server q termino la llamada: lo triggerea cualquiera de los dos
                    Some(ClientMessage::CallEnd { username, target_user }) => {
                        {
                            let mut busy_map = busy_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                            busy_map.remove(&username);
                            busy_map.remove(&target_user);
                        }
                        let clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        if let Some(target_stream) = clients_map.get(&target_user) {
                            let response = ServerMessage::CallEnded { from_user: username.clone() };
                            if let Ok(mut tls_s) = target_stream.lock() {
                                let _ = write_all_nonblocking(&mut *tls_s, response.to_json_line().as_bytes());
                                let _ = tls_s.flush();
                            }
                        }
                    }
                    Some(ClientMessage::CallReject { username, target_user }) => {
                        let mut clients_map = connected_clients.lock().map_err(|e| std::io::Error::other(format!("Sync Error: {e}")))?;
                        if let Some(target_stream) = clients_map.get_mut(&target_user) {
                            let response = ServerMessage::CallRejected { from_user: username.clone() };
                            if let Ok(mut tls_s) = target_stream.lock() {
                                let _ = write_all_nonblocking(&mut *tls_s, response.to_json_line().as_bytes());
                                let _ = tls_s.flush();
                            }
                        }
                    }
                    None => {
                        eprintln!("[ERROR] Failed to parse JSON");
                        let response = ServerMessage::Error { reason: "Invalid JSON".to_string() };
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}"))),
                        };
                        let _ = write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                        let _ = tls_write.flush();
                    }
                }
            }
        }
    }
    Ok(())
}