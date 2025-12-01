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

#[derive(Debug, Clone)]
struct StoredOffer {
    from_user: String,
    offer: String,
}

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
    println!("\n\n[SERVER] Starting on {}", server_info.server_adder);

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
    let login = ServerUserData::new(file_path).unwrap();

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
                eprintln!("[ERROR] Connection error: {}", e);
            }
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

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

    let mut buffer = [0u8; 4096];
    // Loop infinito: leer múltiples mensajes del mismo cliente
    loop {
        let mut guard_read = match read_tls.lock() {
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
                continue;
            }
            Err(e) => {
                eprintln!("[ERROR] Read error: {}", e);
                break;
            }
        };

        drop(guard_read);

        if n == 0 {
            println!("[SERVER] Client disconnected (socket closed)");

            if let Some(user) = &current_user {
                {
                    // Dejar los cochetes es para que se dropee el lock cuando salga del scope
                    // Sacar del mapa de conexiones
                    let mut clients_map = connected_clients.lock().unwrap();
                    clients_map.remove(user);

                    //de haber estado ocupado sacar tambien de ahi
                    let mut busy_map = busy_clients.lock().unwrap();
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
                    eprintln!("[SERVER] Removed {} and notified others", user);
                }
                // Desconectar el client
                let mut clients = active_clients.lock().unwrap();
                let _ = clients.desconectar(user);
            }

            break;
        }

        println!("[SERVER] Before utf-8 Conversion: {:?} bytes", &n);

        let line = match String::from_utf8(buffer[..n].to_vec()) {
            Ok(l) => l,
            Err(e) => {
                return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
            }
        };

        println!("[SERVER] Line Read: {:?}", line);

        match ClientMessage::from_json(&line) {
            Some(ClientMessage::Login { username, password }) => {
                let mut clients = active_clients.lock().unwrap();
                if clients.verificar_login(&username, &password).is_ok() {
                    // Marcar como conectado en la BD
                    let _ = clients.conectar(&username);
                    let clients_list: Vec<String>;
                    eprintln!("[SERVER] User '{}' logged in", username);
                    let msg_joined = ServerMessage::UserJoined {
                        username: username.clone(),
                    };
                    let msg_json = msg_joined.to_json_line();

                    //scope para evitar lock
                    {
                        let mut clients_map = connected_clients.lock().unwrap();
                        clients_map.insert(username.clone(), write_tls.clone());
                        clients_list = clients_map.keys().cloned().collect();
                    }

                    {
                        let clients_map = connected_clients.lock().unwrap();
                        for (other_user, other_stream) in clients_map.iter() {
                            println!("[SERVER] loop other user: {:?}", &other_user);
                            if other_user != &username {
                                if let Ok(mut tls_s) = other_stream.lock() {
                                    eprintln!("\n\n[SERVER] Login Success");
                                    let _ = write_all_nonblocking(&mut *tls_s, msg_json.as_bytes());
                                    let _ = tls_s.flush();
                                }
                            }
                        }
                    }

                    {
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => {
                                return Err(std::io::Error::other(format!(
                                    "TLS mutex poisoned: {e}"
                                )));
                            }
                        };
                        println!("\n\n[SERVER] Login Success");
                        let response = ServerMessage::LoginOk {
                            username: username.clone(),
                        };
                        let _ = write_all_nonblocking(
                            &mut *tls_write,
                            response.to_json_line().as_bytes(),
                        );
                        let _ = tls_write.flush();
                        println!("[SERVER] Sent LoginOk\n");
                    }

                    // Le enviamos la lista una vez que entra, ya que la primera vez
                    // el usuario todavia no estaba adentro
                    {
                        let user_list = ServerMessage::UserList {
                            users: clients_list,
                        };
                        let list_json = user_list.to_json_line();
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => {
                                return Err(std::io::Error::other(format!(
                                    "TLS mutex poisoned: {e}"
                                )));
                            }
                        };
                        let _ = write_all_nonblocking(&mut *tls_write, list_json.as_bytes());
                        let _ = tls_write.flush();
                        println!("[SERVER] Sent Initial UserList to new user");
                    }
                } else {
                    let response = ServerMessage::LoginError {
                        reason: "Datos invalidos o usuario ya conectado".to_string(),
                    };
                    let mut tls_write = match write_tls.lock() {
                        Ok(s) => s,
                        Err(e) => {
                            return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                        }
                    };

                    let _ =
                        write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                    let _ = tls_write.flush();
                    println!("[SERVER] Login failed for '{}'", username);
                }
            }
            Some(ClientMessage::CallStarted {
                username,
                target_user,
            }) => {
                eprintln!("[SERVER] CallStarted from {} to {}", username, target_user);

                // checkeo si alguno de los dos ya está ocupado
                let mut busy_map = busy_clients.lock().unwrap();
                if busy_map.contains_key(&target_user) || busy_map.contains_key(&username) {
                    eprintln!("[SERVER] Busy user. Notifying caller {}", username);
                    let response = ServerMessage::CalledUserBusy;

                    let mut tls_write = match write_tls.lock() {
                        Ok(s) => s,
                        Err(e) => {
                            return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                        }
                    };
                    let _ =
                        write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                    tls_write.flush()?;
                    continue;
                }

                // Si ambos están libres, agarro streams y los marco como ocupados.
                let clients_map = connected_clients.lock().unwrap();
                let user1 = clients_map.get(&username);
                let user2 = clients_map.get(&target_user);

                // Guardar en ocupado solo si ambos existen en conexiones activas para
                //asegurarnos que no se rompa nada
                if let (Some(user1), Some(user2)) = (user1, user2) {
                    busy_map.insert(username.clone(), user1.clone());
                    busy_map.insert(target_user.clone(), user2.clone());
                    eprintln!("[SERVER] Marked {} and {} as busy", username, target_user);
                }
            }

            Some(ClientMessage::SignUp { username, password }) => {
                // Tomo la lista de clientes
                let mut clients = active_clients.lock().unwrap();

                match clients.verificar_signup(&username, &password) {
                    Ok(()) => {
                        // Agrego el usuario al archivo json
                        let _ = clients.add_user(&username, &password);
                        // Como las credenciales son válidas, genero la sesión inmediatamente después

                        // ----- Reuso bloque de login -----
                        let _ = clients.conectar(&username);
                        let clients_list: Vec<String>;
                        eprintln!("[SERVER] User '{}' logged in", username);

                        {
                            let mut clients_map = connected_clients.lock().unwrap();

                            // Server avisa a los ya conectados que entró uno nuevo (avisa a todos los Observers)
                            let msg_joined = ServerMessage::UserJoined {
                                username: username.clone(),
                            };
                            let msg_json = msg_joined.to_json_line();

                            for (other_user, other_stream) in clients_map.iter_mut() {
                                println!("[SERVER] loop other user: {:?}", &other_user);

                                let _tls_other_stream = match other_stream.lock() {
                                    Ok(mut tls_s) => {
                                        if other_user != &username {
                                            eprintln!("\n\n[SERVER] Login Success");
                                            // let _ = tls_s.write_all(msg_json.as_bytes());
                                            let _ = write_all_nonblocking(
                                                &mut *tls_s,
                                                msg_json.as_bytes(),
                                            );
                                            let _ = tls_s.flush();
                                        }
                                        tls_s
                                    }
                                    Err(e) => {
                                        return Err(std::io::Error::other(format!(
                                            "TLS mutex poisoned: {e}"
                                        )));
                                    }
                                };
                            }

                            // Guardar la conexión de este usuario
                            // if let Ok(write_clone) = stream_write.try_clone() {
                            clients_map.insert(username.clone(), write_tls.clone());
                            // }
                            clients_list = clients_map.keys().cloned().collect();
                        }

                        current_user = Some(username.clone());

                        // Avisar a la UI que el login fue exitoso
                        // Enviamos primero LoginOk para que UI cambie de vista al Lobby
                        // para después cuando reciba la lista de conectados, lo pueda mostrar correctamente
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => {
                                return Err(std::io::Error::other(format!(
                                    "TLS mutex poisoned: {e}"
                                )));
                            }
                        };
                        println!("\n\n[SERVER] Login Success");
                        let response = ServerMessage::LoginOk { username };
                        let response_json = response.to_json_line();
                        // stream_write.flush()?; //me aseguro que se escriba todo
                        let _ = write_all_nonblocking(&mut *tls_write, response_json.as_bytes());
                        tls_write.flush()?;
                        println!("[SERVER] Sent LoginOk\n");

                        // Le enviamos la lista una vez que entra, ya que la primera vez
                        // el usuario todavia no estaba adentro
                        let user_list = ServerMessage::UserList {
                            users: clients_list,
                        };
                        let list_json = user_list.to_json_line();
                        let _ = write_all_nonblocking(&mut *tls_write, list_json.as_bytes());
                        tls_write.flush()?;
                        println!("[SERVER] Sent Initial UserList to new user");
                        // ----- Reuso bloque de login -----
                    }
                    Err(e) => {
                        // Creo un mensaje para la app con la razón del error
                        let response = ServerMessage::SignUpError {
                            reason: e.to_string(),
                        };
                        let response_json = response.to_json_line();

                        // Envío mensaje a la app por TLS
                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => {
                                return Err(std::io::Error::other(format!(
                                    "TLS mutex poisoned: {e}"
                                )));
                            }
                        };

                        let _ = write_all_nonblocking(&mut *tls_write, response_json.as_bytes());
                        tls_write.flush()?;
                        println!("[SERVER] Login failed for '{}'", username);
                    }
                }
            }

            Some(ClientMessage::Logout { username }) => {
                {
                    let mut clients = active_clients.lock().unwrap();
                    let _ = clients.desconectar(&username);
                }

                eprintln!("[SERVER] User {} logged out", &username);

                // Enviar confirmación
                let response = ServerMessage::LogoutOk;
                let response_json = response.to_json_line();

                let mut tls_write = match write_tls.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                    }
                };

                let _ = write_all_nonblocking(&mut *tls_write, response_json.as_bytes());
                tls_write.flush()?;

                // Sacar la conexión de los conectados y notificar salida
                {
                    let mut clients_map = connected_clients.lock().unwrap();
                    clients_map.remove(&username);

                    //tambien lo saco de ocupado de haberlo estado
                    let mut busy_map = busy_clients.lock().unwrap();
                    busy_map.remove(&username);

                    let msg_left = ServerMessage::UserLeft {
                        username: username.clone(),
                    };
                    let msg_json = msg_left.to_json_line();
                    for (_, other_stream) in clients_map.iter_mut() {
                        let _tls_write = match other_stream.lock() {
                            Ok(mut s) => {
                                let _ = write_all_nonblocking(&mut *s, msg_json.as_bytes());
                                let _ = s.flush();
                                s
                            }
                            Err(e) => {
                                return Err(std::io::Error::other(format!(
                                    "TLS mutex poisoned: {e}"
                                )));
                            }
                        };
                    }
                }
                //La aplicacion en este socket no tiene usuario
                //No cierro socket para agilizar si se vuelve a conectar -> consultar
                current_user = None;
            }

            Some(ClientMessage::GetUsers {}) => {
                // lockeamos para leerlo y evitar inconsistencias
                let clients_guard = connected_clients.lock().unwrap();

                let users_vec: Vec<String> = clients_guard.keys().cloned().collect();
                drop(clients_guard);

                let response = ServerMessage::UserList { users: users_vec };
                let response_json = response.to_json_line();

                let mut tls_write = match write_tls.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                    }
                };

                let _ = write_all_nonblocking(&mut *tls_write, response_json.as_bytes());
                tls_write.flush()?;
                println!("[SERVER] Sent users list (from active connections)\n");
            }

            //quiero llamar a alguien
            Some(ClientMessage::Offer {
                username,
                user_id: _,
                password: _,
                offer,
                target_user,
            }) => {
                eprintln!("[SERVER] OFFER from {}", username);

                // Almacenar el offer recibido
                {
                    let mut offers_map = stored_offers.lock().unwrap();
                    let stored = StoredOffer {
                        from_user: username.clone(),
                        offer: offer.clone(),
                    };
                    offers_map.insert(username.clone(), stored);
                }

                // Reenviar el offer al client al que me quiero conectar
                if let Some(target) = target_user {
                    let busy_map = busy_clients.lock().unwrap();
                    if busy_map.contains_key(&target) {
                        eprintln!(
                            "[SERVER] Target {} is busy. Notifying caller {}",
                            target, username
                        );
                        let response = ServerMessage::CalledUserBusy;

                        let mut tls_write = match write_tls.lock() {
                            Ok(s) => s,
                            Err(e) => {
                                return Err(std::io::Error::other(format!(
                                    "TLS mutex poisoned: {e}"
                                )));
                            }
                        };

                        let _ = write_all_nonblocking(
                            &mut *tls_write,
                            response.to_json_line().as_bytes(),
                        );
                        tls_write.flush()?;
                    } else {
                        let mut clients_map = connected_clients.lock().unwrap();
                        if let Some(target_stream) = clients_map.get_mut(&target) {
                            let response = ServerMessage::OfferForwarded {
                                from_user: username.clone(),
                                offer: offer.clone(),
                            };

                            let mut tls_write = match target_stream.lock() {
                                Ok(s) => s,
                                Err(e) => {
                                    return Err(std::io::Error::other(format!(
                                        "TLS mutex poisoned: {e}"
                                    )));
                                }
                            };

                            let _ = write_all_nonblocking(
                                &mut *tls_write,
                                response.to_json_line().as_bytes(),
                            );
                            let _ = tls_write.flush();
                            eprintln!("[SERVER] Forwarded OFFER to {}", target);
                        } else {
                            eprintln!("[SERVER] Target {} not found", target);
                        }
                    }
                }
            }

            // mi respuesta al offer
            Some(ClientMessage::Answer {
                username,
                user_id: _,
                answer,
                target_user,
            }) => {
                eprintln!(
                    "[SERVER] ANSWER from {} -> target: {}",
                    username, target_user
                );

                let mut clients_map = connected_clients.lock().unwrap();
                //  Creo mensaje del answer al client que envió el offer
                if let Some(target_stream) = clients_map.get_mut(&target_user) {
                    let response = ServerMessage::AnswerForwarded {
                        from_user: username.clone(),
                        answer: answer.clone(),
                    };

                    let mut tls_write = match target_stream.lock() {
                        Ok(s) => s,
                        Err(e) => {
                            return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                        }
                    };

                    match write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes())
                    {
                        Ok(()) => {
                            if let Err(e) = tls_write.flush() {
                                eprintln!(
                                    "[SERVER] ERROR flushing ANSWER to {}: {}",
                                    target_user, e
                                );
                            } else {
                                eprintln!("[SERVER] ANSWER bytes sent OK to {}", target_user);
                            }
                        }
                        Err(e) => {
                            eprintln!("[SERVER] ERROR sending ANSWER to {}: {}", target_user, e);
                        }
                    }
                    tls_write.flush()?;
                    eprintln!("[SERVER] Forwarded ANSWER to {}", target_user);
                } else {
                    eprintln!(
                        "[SERVER] ANSWER target {} NOT FOUND in connected_clients",
                        target_user
                    );
                }
            }

            //aceptar o rechazar llamada
            Some(ClientMessage::RequestOffer {
                username: _,
                target_user,
            }) => {
                let offers_map = stored_offers.lock().unwrap();

                let mut tls_write = match write_tls.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                    }
                };

                if let Some(stored) = offers_map.get(&target_user) {
                    let response = ServerMessage::OfferAvailable {
                        from_user: stored.from_user.clone(),
                        offer: stored.offer.clone(),
                    };
                    let response_json = response.to_json_line();

                    let _ = write_all_nonblocking(&mut *tls_write, response_json.as_bytes());
                    tls_write.flush()?;
                } else {
                    let response = ServerMessage::Error {
                        reason: format!("No offer available from {}", target_user),
                    };
                    let _ =
                        write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                }
            }

            //aviso al server q termino la llamada: lo triggerea cualquiera de los dos
            Some(ClientMessage::CallEnd {
                username,
                target_user,
            }) => {
                eprintln!("[SERVER] CallEnd by: {} to: {}", username, target_user);

                //saco a username de ocupado
                {
                    let mut busy_map = busy_clients.lock().unwrap();
                    busy_map.remove(&username);
                }

                let clients_map = connected_clients.lock().unwrap();
                if let Some(target_stream) = clients_map.get(&target_user) {
                    //Ese usuario tambien va a dejar de estar ocupado
                    {
                        let mut busy_map = busy_clients.lock().unwrap();
                        busy_map.remove(&target_user);
                    }

                    let response = ServerMessage::CallEnded {
                        from_user: username.clone(),
                    };

                    let mut tls_write = match target_stream.lock() {
                        Ok(s) => s,
                        Err(e) => {
                            return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                        }
                    };

                    let _ =
                        write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                    tls_write.flush()?;
                    eprintln!("[SERVER] Forwarded CallEnded to {}", target_user);
                } else {
                    eprintln!("[SERVER] Target not found for CallEnded");
                }
            }

            // Mensaje para avisar al client que inició la llamada
            // target_user: Al que se le rechazó la llamada (el client que no inició la llamada)
            Some(ClientMessage::CallReject {
                username,
                target_user,
            }) => {
                eprintln!(
                    "[SERVER] CallReject from {} -> target: {}",
                    username, target_user
                );

                let mut clients_map = connected_clients.lock().unwrap();
                if let Some(target_stream) = clients_map.get_mut(&target_user) {
                    // Notificamos al client que llamó que fue rechazado
                    let response = ServerMessage::CallRejected {
                        from_user: username.clone(),
                    };

                    let mut tls_write = match target_stream.lock() {
                        Ok(s) => s,
                        Err(e) => {
                            return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                        }
                    };

                    let _ =
                        write_all_nonblocking(&mut *tls_write, response.to_json_line().as_bytes());
                    let _ = tls_write.flush();
                    eprintln!("[SERVER] Forwarded CallRejected to {}", target_user);
                } else {
                    eprintln!("[SERVER] Target {} not found for CallReject", target_user);
                }
            }

            None => {
                //no deberia pasar, se recibe un mensaje que no se reconoce
                eprintln!("[ERROR] Failed to parse JSON");
                let response = ServerMessage::Error {
                    reason: "Invalid JSON".to_string(),
                };

                let mut tls_write = match write_tls.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(std::io::Error::other(format!("TLS mutex poisoned: {e}")));
                    }
                };
                let response_json = response.to_json_line();

                let _ = write_all_nonblocking(&mut *tls_write, response_json.as_bytes());
                let _ = tls_write.flush();
            }
        }
    }

    Ok(())
}
