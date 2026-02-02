use std::io::{ErrorKind, Read};
use std::net::{TcpStream, UdpSocket};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::audio_processing::AudioSystem;
use crate::camera::Camera;
use crate::client::Client;
use crate::client_errors::ClientErrors;
use crate::config::AppParameters;
use crate::file_path::FilePathPacket;
use crate::ice::{crear_conexion_ice, elegir_codec};
use crate::log_c;
use crate::sdp_errors::SdpError;
use crate::sdp_protocol::sdp_handshake;
use crate::signalling_protocol::{ClientMessage, ServerMessage, write_all_nonblocking};
use crate::tls_interface::client_tls_stream;
use crate::vista::call_end::VistaCallEnd;
use crate::vista::call_start::VistaCallStart;
use crate::vista::called_user_busy::VistaCalledUserBusy;
use crate::vista::incoming_call::VistaIncomingCall;
use crate::vista::lobby::VistaLobby;
use crate::vista::sign_up::VistaSignUp;
use crate::vista::{Vista, cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin};
use eframe::egui;
use opencv::core::Mat;
use rustls::StreamOwned;

// COMENTARIO: Dsp se borra esto
// Puse comentarios en las partes que modifiqué para que puedan saber donde están

pub enum AppEvent {
    LoginSuccess(String),
    LoginError(String),
    SignUpSuccess(String),
    SignUpError(String),
    UserListUpdated(Vec<String>),
    NewUser(String),
    UserLeft(String),
    SdpMessage(ServerMessage),
    ConnectionLost,
    CalledUserBusy,
    FileSendingRequest { from_user: String, filename: String },
    FileSendingAccepted { from_user: String, filename: String },
    FileSendingRejected { from_user: String, filename: String },
}

pub struct AppUI {
    vista_actual: Vista,
    vista_prev: Option<Box<Vista>>,
    camara: Option<Arc<Mutex<Camera>>>,
    server_events_rx: Receiver<AppEvent>,
    server_writer: Option<Arc<std::sync::Mutex<StreamOwned<rustls::ClientConnection, TcpStream>>>>,
    sdp_handshake_tx: Option<Sender<ServerMessage>>,
    pedido_socket: Option<Receiver<Result<(UdpSocket, Vec<String>, Vec<String>), SdpError>>>,
    handshake_handle: Option<JoinHandle<Result<(), ClientErrors>>>,
    handles: Vec<JoinHandle<()>>,
    pending_offer: Option<String>,
    app_params: AppParameters,
    cliente: Option<Client>,
    client_handles: Result<Vec<JoinHandle<Result<(), ClientErrors>>>, ClientErrors>,
    rx_remoto: Option<Receiver<Mat>>,
    tx_file: Option<SyncSender<FilePathPacket>>, // CHANNEL agregado para enviar FilePath al Client
    in_call: Arc<AtomicBool>,
    current_username: Option<String>,
    target_username: Option<String>,

    file_path: Option<FilePathPacket>, // Para guardar el FilePath

    audio_system: Arc<AudioSystem>,
    _input_stream: cpal::Stream,
    _output_stream: cpal::Stream,
}

impl Drop for AppUI {
    // Manejo de handles de threads
    fn drop(&mut self) {
        if let Some(username) = &self.current_username
            && let Some(writer) = &mut self.server_writer
        {
            let msg = ClientMessage::Logout {
                username: username.clone(),
            };

            let writer_arc = Arc::clone(writer);
            let write_handle = std::thread::spawn(move || {
                let mut guard_write = match writer_arc.lock() {
                    Ok(gw) => gw,
                    Err(_) => return,
                };

                let _ = write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
            });
            self.handles.push(write_handle);
        }
        self.liberar_recursos();
        self.liberar_handles();
    }
}

impl AppUI {
    pub fn new(app_params: AppParameters) -> Option<Self> {
        if let Ok(stream) = TcpStream::connect(&app_params.sdp_handshake.server_adder) {
            // Encriptar TcpStream
            let tls_stream = match client_tls_stream(stream, &app_params.general_info.server_adder)
            {
                Ok(tls_s) => tls_s,
                Err(_) => return None,
            };

            // Setear el TcpStream envuelto como non_blocking, no es lo ideal, pero para como manejamos el envío y recepción tomamos esta decisión
            let _ = tls_stream.get_ref().set_nonblocking(true);
            // Envuelvo en un Arc<Mutex<..>> para poder después clonar
            let tls_arc = Arc::new(Mutex::new(tls_stream));
            let read_tls = tls_arc.clone();
            let write_tls = tls_arc.clone();
            println!("[CLIENT] Starting encripted transmission\n");
            log_c!("[CLIENT] Starting encripted transmission\n");

            let (tx, rx) = channel::<AppEvent>();

            // Thread para escuchar del servidor
            let _read_handle = std::thread::spawn(move || {
                // Buffer acumulativo para reconstruir mensajes fragmentados
                let mut pending_buffer: Vec<u8> = Vec::new();

                loop {
                    // Setear un Buffer para leer
                    let mut buffer = [0u8; 4096];
                    // Lockear, leer y guardar la cantidad de bytes que se leyó
                    let n = {
                        let mut guard_read = match read_tls.lock() {
                            Ok(gr) => gr,
                            Err(_) => {
                                let _ = tx.send(AppEvent::ConnectionLost);
                                break;
                            }
                        };

                        match guard_read.read(&mut buffer) {
                            Ok(0) => {
                                let _ = tx.send(AppEvent::ConnectionLost);
                                break;
                            }
                            // Si está bloqueado seguir sin quedar bloqueada esperando
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => 0,
                            Ok(n) => n,
                            Err(_) => {
                                let _ = tx.send(AppEvent::ConnectionLost);
                                break;
                            }
                        }
                    };

                    if n > 0 {
                        // Acumular bytes leídos
                        pending_buffer.extend_from_slice(&buffer[..n]);

                        // Procesar todos los mensajes completos (delimitados por \n)
                        while let Some(pos) = pending_buffer.iter().position(|&x| x == b'\n') {
                            let line_bytes: Vec<u8> = pending_buffer.drain(..=pos).collect();

                            if let Ok(line) = String::from_utf8(line_bytes)
                                && let Some(msg) = ServerMessage::from_json(&line)
                            {
                                match msg {
                                    ServerMessage::LoginOk { username } => {
                                        let _ = tx.send(AppEvent::LoginSuccess(username));
                                    }
                                    ServerMessage::LoginError { reason } => {
                                        let _ = tx.send(AppEvent::LoginError(reason));
                                    }
                                    ServerMessage::SignUpOk { username } => {
                                        let _ = tx.send(AppEvent::SignUpSuccess(username));
                                    }
                                    ServerMessage::SignUpError { reason } => {
                                        let _ = tx.send(AppEvent::SignUpError(reason));
                                    }
                                    ServerMessage::UserList { users } => {
                                        let _ = tx.send(AppEvent::UserListUpdated(users));
                                    }
                                    ServerMessage::UserJoined { username } => {
                                        let _ = tx.send(AppEvent::NewUser(username));
                                    }
                                    ServerMessage::UserLeft { username } => {
                                        let _ = tx.send(AppEvent::UserLeft(username));
                                    }
                                    ServerMessage::OfferForwarded { .. } => {
                                        let _ = tx.send(AppEvent::SdpMessage(msg));
                                    }
                                    ServerMessage::AnswerForwarded { .. } => {
                                        let _ = tx.send(AppEvent::SdpMessage(msg));
                                    }
                                    ServerMessage::OfferAvailable { .. } => {
                                        let _ = tx.send(AppEvent::SdpMessage(msg));
                                    }
                                    ServerMessage::CallEnded { .. } => {
                                        let _ = tx.send(AppEvent::SdpMessage(msg));
                                    }
                                    ServerMessage::CallRejected { .. } => {
                                        let _ = tx.send(AppEvent::SdpMessage(msg));
                                    }
                                    ServerMessage::CalledUserBusy => {
                                        let _ = tx.send(AppEvent::CalledUserBusy);
                                    }
                                    // Agregados para testear envío de archivos
                                    ServerMessage::FileSendingRequestForwarded {
                                        from_user,
                                        filename,
                                    } => {
                                        let _ = tx.send(AppEvent::FileSendingRequest {
                                            from_user,
                                            filename,
                                        });
                                    }
                                    ServerMessage::FileSendingAccepted {
                                        from_user,
                                        filename,
                                    } => {
                                        let _ = tx.send(AppEvent::FileSendingAccepted {
                                            from_user,
                                            filename,
                                        });
                                    }
                                    ServerMessage::FileSendingRejected {
                                        from_user,
                                        filename,
                                    } => {
                                        let _ = tx.send(AppEvent::FileSendingRejected {
                                            from_user,
                                            filename,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            });

            // Inicializo AudioSystem, y guardo referencias a los streams
            let audio_system = match AudioSystem::new() {
                Ok(s) => s,
                Err(_) => return None,
            };
            let input_stream = match AudioSystem::build_input_stream(Arc::clone(&audio_system)) {
                Ok(i) => i,
                Err(_) => return None,
            };
            let output_stream = match AudioSystem::build_output_stream(Arc::clone(&audio_system)) {
                Ok(i) => i,
                Err(_) => return None,
            };

            Some(Self {
                //Inicia en vista de login por deffault
                vista_actual: Vista::VistaLogin(VistaLogin::new()),
                vista_prev: None,
                server_events_rx: rx,
                server_writer: Some(write_tls),
                sdp_handshake_tx: None,
                cliente: None,
                rx_remoto: None,
                tx_file: None,
                in_call: Arc::new(AtomicBool::new(false)),
                pedido_socket: None,
                handshake_handle: None,
                handles: Vec::new(),
                client_handles: Ok(Vec::new()),
                pending_offer: None,
                app_params,
                camara: None,
                current_username: None,
                target_username: None,

                file_path: None,

                audio_system,
                _input_stream: input_stream,
                _output_stream: output_stream,
            })
        } else {
            None
        }
    }

    fn push_vista(&mut self, next: Vista) {
        let current = std::mem::replace(&mut self.vista_actual, next);
        self.vista_prev = Some(Box::new(current));
    }

    fn pop_vista(&mut self) {
        if let Some(prev) = self.vista_prev.take() {
            self.vista_actual = *prev;
        }
    }

    // Función para la transición de vistas
    fn cambiar_vista(&mut self, cambio: CambioVista) {
        match cambio {
            CambioVista::None => {}
            CambioVista::ToLogin => {
                //ya estaba en otra pantalla (aprete log out), vuelvo a login
                if let Some(username) = &self.current_username
                    && let Some(writer) = &mut self.server_writer
                {
                    let msg = ClientMessage::Logout {
                        username: username.clone(),
                    };

                    let writer_arc = Arc::clone(writer);
                    // Spawnear una thread aparte para escribir para no bloquear la UI
                    let write_handle = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });
                    self.handles.push(write_handle);
                };
                self.liberar_recursos();
                self.current_username = None;
                self.target_username = None;
                self.pending_offer = None;
                self.vista_actual = Vista::VistaLogin(VistaLogin::new());
            }
            CambioVista::ToSignUp => {
                // Solo se llega desde login, cambio la vista
                self.vista_actual = Vista::VistaSignUp(VistaSignUp::new());
            }
            CambioVista::ToLobby { username } => {
                if !username.is_empty() {
                    self.current_username = Some(username);
                }

                if let Some(writer) = &mut self.server_writer {
                    let msg = ClientMessage::GetUsers {};
                    let writer_arc = Arc::clone(writer);
                    let write_thread = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });
                    self.handles.push(write_thread);
                }

                self.in_call.store(false, Ordering::Relaxed);
                self.liberar_handshake_handle();
                self.target_username = None;
                self.pending_offer = None;
                self.crear_camara();

                if let Some(u) = &self.current_username {
                    self.vista_actual = Vista::VistaLobby(VistaLobby::new(u.to_string()));
                } else {
                    self.vista_actual = Vista::VistaLobby(VistaLobby::default());
                }
            }
            CambioVista::ToCallStart { username } => {
                self.target_username = Some(username.clone());
                self.crear_camara();
                self.iniciar_pedido_cliente();

                if let Some(writer) = &mut self.server_writer {
                    let msg = ClientMessage::GetUsers {};
                    let writer_arc = Arc::clone(writer);
                    let write_handle = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });
                    self.handles.push(write_handle);
                }

                self.vista_actual = Vista::VistaCallStart(VistaCallStart::new(username));
            }
            CambioVista::ToIncomingCall { caller } => {
                self.target_username = Some(caller.clone());
                self.vista_actual = Vista::VistaIncomingCall(VistaIncomingCall::new(caller));
            }
            CambioVista::CalledUserBusy => {
                eprintln!("[CLIENT] Called user was busy");
                log_c!("[CLIENT] Called user was busy");
                self.vista_actual = Vista::VistaCalledUserBusy(VistaCalledUserBusy::new());
            }
            CambioVista::CallAccepted { target_user } => {
                eprintln!("[CLIENT] Called accepted from: {}", target_user);
                log_c!("[CLIENT] Called accepted from: {}", target_user);
                self.target_username = Some(target_user.clone());
                self.crear_camara();

                // Iniciamos handshake
                self.iniciar_pedido_cliente();

                // comunicamos la oferta por el channel
                if let (Some(tx), Some(offer)) = (&self.sdp_handshake_tx, &self.pending_offer) {
                    let msg = ServerMessage::OfferForwarded {
                        from_user: target_user.clone(),
                        offer: offer.clone(),
                    };
                    let _ = tx.send(msg);
                }

                //Aviso que ambos usuarios van a estar ocupados
                if let (Some(writer), Some(me)) = (&mut self.server_writer, &self.current_username)
                {
                    let msg = ClientMessage::CallStarted {
                        username: me.clone(),
                        target_user: target_user.clone(),
                    };
                    let writer_arc = Arc::clone(writer);
                    let write_thread = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });
                    self.handles.push(write_thread);
                }

                self.vista_actual = Vista::VistaCallStart(VistaCallStart::new(target_user));
            }
            CambioVista::CallRejected { target_user } => {
                eprintln!("[CLIENT] Rejected call from {}", target_user);
                log_c!("[CLIENT] Rejected call from {}", target_user);
                if let (Some(writer), Some(me)) = (&mut self.server_writer, &self.current_username)
                {
                    //hay que comunicarle al otro cliente
                    let msg = ClientMessage::CallReject {
                        username: me.clone(),
                        target_user: target_user.clone(),
                    };
                    let writer_arc = Arc::clone(writer);
                    let write_handle = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });
                    self.handles.push(write_handle);
                }
                self.pending_offer = None;
                self.cambiar_vista(CambioVista::ToLobby {
                    username: String::new(),
                });
            }
            CambioVista::ToCall => {
                if let Some(c) = &self.cliente {
                    let stats = c.copy_stats();

                    self.vista_actual = Vista::VistaLlamada(VistaLlamada::new(
                        self.rx_remoto.take(),
                        Arc::clone(&self.in_call),
                        stats,
                    ))
                }
            }
            CambioVista::ToCallEnd => {
                if let (Some(user), Some(target)) = (&self.current_username, &self.target_username)
                {
                    // Si cortamos nosotros, avisar
                    if let Some(writer) = &mut self.server_writer {
                        //le aviso al otro usuario
                        let msg = ClientMessage::CallEnd {
                            username: user.clone(),
                            target_user: target.clone(),
                        };
                        let writer_arc = Arc::clone(writer);
                        let write_handle = std::thread::spawn(move || {
                            let mut guard_write = match writer_arc.lock() {
                                Ok(gw) => gw,
                                Err(_) => return,
                            };

                            let _ = write_all_nonblocking(
                                &mut *guard_write,
                                msg.to_json_line().as_bytes(),
                            );
                        });
                        self.handles.push(write_handle);
                    }
                }
                self.liberar_recursos();
                self.audio_system.disconnect_audio();
                self.crear_camara();
                self.vista_actual = Vista::VistaCallEnd(VistaCallEnd::new());
            }
            CambioVista::SendFile => {
                self.push_vista(Vista::VistaSendFile(
                    crate::vista::file_sending::VistaSendFile::new(),
                ));
            }
            CambioVista::SendFileQuit => {
                self.pop_vista();
            }
            CambioVista::SendFileConfirmed(path) => {
                if let (Some(user), Some(target)) = (&self.current_username, &self.target_username)
                    && let Some(writer) = &mut self.server_writer
                {
                    let msg = ClientMessage::FileSendingRequest {
                        username: user.clone(),
                        target_user: target.clone(),
                        filename: path.clone(),
                    };
                    // Se genera acá el FilePathPacket con el FilePath
                    self.file_path = Some(FilePathPacket::new(
                        Some(PathBuf::from(path.clone())),
                        false,
                    ));

                    let writer_arc = Arc::clone(writer);
                    let write_handle = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });

                    self.handles.push(write_handle);
                };

                self.pop_vista();
                // if let Some(c) = &self.cliente {
                //     let stats = c.copy_stats();

                //     self.vista_actual = Vista::VistaLlamada(VistaLlamada::new(
                //         self.rx_remoto.take(),
                //         Arc::clone(&self.in_call),
                //         stats,
                //     ))
                // }
            }
            CambioVista::SendFileRequested {
                from_user,
                filename,
            } => {
                self.push_vista(Vista::VistaFileSendingRequested(from_user, filename));
            }
            CambioVista::SendFileAccepted {
                from_user: _,
                filename,
            } => {
                if let (Some(user), Some(target)) = (&self.current_username, &self.target_username)
                    && let Some(writer) = &mut self.server_writer
                {
                    let msg = ClientMessage::FileSendingAccept {
                        username: user.clone(),
                        target_user: target.clone(),
                        filename: filename.clone(),
                    };

                    // self.file_path = Some(FilePathPacket::new(None, true));
                    // if let Some(file_pkt_ref) = &self.file_path {
                    //     if let Some(tx_file) = &self.tx_file {
                    //         tx_file.send(file_pkt_ref.clone());
                    //     }
                    // }

                    let writer_arc = Arc::clone(writer);
                    let write_handle = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });

                    self.handles.push(write_handle);
                };

                self.pop_vista();
            }
            CambioVista::SendFileRejected {
                from_user: _,
                filename,
            } => {
                if let (Some(user), Some(target)) = (&self.current_username, &self.target_username)
                    && let Some(writer) = &mut self.server_writer
                {
                    let msg = ClientMessage::FileSendingReject {
                        username: user.clone(),
                        target_user: target.clone(),
                        filename: filename.clone(),
                    };

                    let writer_arc = Arc::clone(writer);
                    let write_handle = std::thread::spawn(move || {
                        let mut guard_write = match writer_arc.lock() {
                            Ok(gw) => gw,
                            Err(_) => return,
                        };

                        let _ =
                            write_all_nonblocking(&mut *guard_write, msg.to_json_line().as_bytes());
                    });

                    self.handles.push(write_handle);
                };

                self.pop_vista();
            }
            CambioVista::TargetUserSendFileAccepted {
                from_user,
                filename,
            } => {
                self.push_vista(Vista::VistaTargetUserFileSendingAccepted(
                    from_user, filename,
                ));
            }
            CambioVista::TargetUserSendFileRejected {
                from_user,
                filename,
            } => {
                self.push_vista(Vista::VistaTargetUserFileSendingRejected(
                    from_user, filename,
                ));
            }
            CambioVista::SendFileNoticeClosed => {
                self.pop_vista();
            }
            CambioVista::CamEnable => {
                self.iniciar_camara();
            }
            CambioVista::CamDisable => {
                self.detener_camara();
            }
            CambioVista::MicEnable => {
                self.iniciar_mic();
            }
            CambioVista::MicDisable => {
                self.detener_mic();
            }
            CambioVista::PlaybackToggle { state } => {
                self.audio_system.set_mic_test(state);
            }
        }
    }

    fn iniciar_mic(&self) {
        self.audio_system.set_capture(true);
    }

    fn detener_mic(&self) {
        self.audio_system.set_capture(false);
    }

    fn liberar_recursos(&mut self) {
        self.cliente = None;
        self.rx_remoto = None;
        self.in_call.store(false, Ordering::Relaxed);
        self.pedido_socket = None;
        self.sdp_handshake_tx = None;
        self.liberar_handshake_handle();
        self.liberar_client_handles();
        self.liberar_camara();
        self.pending_offer = None;
    }

    fn crear_camara(&mut self) {
        if self.camara.is_some() {
            return;
        }

        let mut camara = Camera::new();
        camara.enable_camera();
        self.camara = Some(Arc::new(Mutex::new(camara)));
    }

    fn iniciar_camara(&mut self) {
        if let Some(cam) = &self.camara
            && let Ok(mut c) = cam.lock()
        {
            c.enable_camera();
        }
    }

    fn detener_camara(&mut self) {
        if let Some(cam) = &self.camara
            && let Ok(mut c) = cam.lock()
        {
            c.disable_camera();
        }
    }

    fn liberar_camara(&mut self) {
        self.camara = None;
    }

    fn iniciar_pedido_cliente(&mut self) {
        self.in_call.store(true, Ordering::SeqCst);
        let handshake_params = self.app_params.sdp_handshake.clone();
        let in_call = Arc::clone(&self.in_call);
        let username = self.current_username.clone().unwrap_or_default();
        let target_user = self.target_username.clone().unwrap_or_default();

        let (sdp_tx, sdp_rx) = channel::<ServerMessage>();
        self.sdp_handshake_tx = Some(sdp_tx);

        if let Some(writer) = &self.server_writer {
            let mut writer_clone = writer.clone();
            let (tx_result, rx_result) = std::sync::mpsc::channel();

            let handshake_handle = std::thread::spawn(move || -> Result<(), ClientErrors> {
                eprintln!("[DEBUG THREAD] Starting sdp_handshake...");
                log_c!("[DEBUG THREAD] Starting sdp_handshake...");
                let (sockets, local_candidates, remote_ice, codecs) = match sdp_handshake(
                    in_call,
                    handshake_params,
                    username,
                    target_user,
                    &mut writer_clone,
                    sdp_rx,
                ) {
                    Err(e) => {
                        let _ = tx_result.send(Err(e.clone()));
                        return Err(ClientErrors::SdpError(e));
                    }
                    Ok(info) => info,
                };
                let _codec = match elegir_codec(&codecs) {
                    Err(e) => return Err(ClientErrors::IceError(e)),
                    Ok(c) => c,
                };
                let socket = match crear_conexion_ice(sockets, &remote_ice) {
                    Err(e) => return Err(ClientErrors::IceError(e)),
                    Ok(s) => s,
                };
                if let Err(e) = tx_result.send(Ok((socket, local_candidates, remote_ice))) {
                    eprintln!("[CLIENT] ERROR sending UDP socket through channel: {:?}", e);
                    log_c!("[CLIENT] ERROR sending UDP socket through channel: {:?}", e);
                };
                Ok(())
            });
            self.pedido_socket = Some(rx_result);
            self.handshake_handle = Some(handshake_handle);
        }
    }

    fn crear_cliente(&mut self, socket: UdpSocket, is_active: bool) {
        let (tx_remoto, rx_remoto) = std::sync::mpsc::sync_channel(1);
        let (tx_remoto_file, rx_remoto_file) = std::sync::mpsc::sync_channel::<FilePathPacket>(1);
        self.rx_remoto = Some(rx_remoto);
        self.tx_file = Some(tx_remoto_file); // Sender para archivo
        if let Some(cam) = &self.camara {
            let cliente = Client::new(
                tx_remoto,
                rx_remoto_file,
                Arc::clone(&self.in_call),
                Arc::clone(cam),
            );
            if let Ok(mut c) = cliente {
                let _ = c.add_peer(socket);
                c.set_rol(is_active);
                self.client_handles = c.initialize_call(&self.audio_system);
                self.cliente = Some(c);
            }
        }
    }

    fn liberar_handles(&mut self) {
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }

    fn liberar_client_handles(&mut self) {
        if let Ok(handles) = &mut self.client_handles {
            while let Some(h) = handles.pop() {
                let _ = h.join();
            }
        }
    }
    fn liberar_handshake_handle(&mut self) {
        if let Some(rx) = &self.pedido_socket {
            if let Some(h) = self.handshake_handle.take() {
                let _ = h.join();
            }
            let _ = rx.try_recv();
        }
    }
}

impl eframe::App for AppUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.4);

        while let Ok(event) = self.server_events_rx.try_recv() {
            match event {
                AppEvent::LoginSuccess(username) => {
                    self.cambiar_vista(CambioVista::ToLobby { username });
                }
                AppEvent::LoginError(reason) => {
                    if let Vista::VistaLogin(login) = &mut self.vista_actual {
                        login.error_msg = Some(reason);
                        login.auth_pending = false;
                    }
                }
                AppEvent::SignUpSuccess(username) => {
                    self.cambiar_vista(CambioVista::ToLobby { username });
                }
                AppEvent::SignUpError(reason) => {
                    if let Vista::VistaSignUp(sign_up) = &mut self.vista_actual {
                        sign_up.username = "".to_string();
                        sign_up.password = "".to_string();
                        sign_up.error_msg = Some(reason);
                        sign_up.auth_pending = false;
                    }
                }
                AppEvent::UserListUpdated(users) => {
                    if let Vista::VistaLobby(lobby) = &mut self.vista_actual {
                        lobby.users_list = users
                            .into_iter()
                            .filter(|u| Some(u) != self.current_username.as_ref())
                            .collect();
                    }
                }
                AppEvent::NewUser(user) => {
                    if let Vista::VistaLobby(lobby) = &mut self.vista_actual
                        && Some(&user) != self.current_username.as_ref()
                        && !lobby.users_list.contains(&user)
                    {
                        lobby.users_list.push(user);
                    }
                }

                AppEvent::CalledUserBusy => {
                    self.cambiar_vista(CambioVista::CalledUserBusy);
                }

                AppEvent::UserLeft(user) => {
                    if let Vista::VistaLobby(lobby) = &mut self.vista_actual {
                        lobby.users_list.retain(|u| u != &user);
                    }
                }
                AppEvent::SdpMessage(msg) => {
                    match msg {
                        // Tenemos un offer
                        ServerMessage::OfferForwarded { from_user, offer } => {
                            if matches!(self.vista_actual, Vista::VistaLobby(_)) {
                                eprintln!("[AppUI] Incomming call from {}", from_user);
                                log_c!("[AppUI] Incomming call from {}", from_user);
                                // Guardo la oferta
                                self.pending_offer = Some(offer);
                                self.cambiar_vista(CambioVista::ToIncomingCall {
                                    caller: from_user,
                                });
                            } else if let Some(tx) = &self.sdp_handshake_tx {
                                // Si ya estamos en handshake, pasamos al thread
                                let _ = tx.send(ServerMessage::OfferForwarded { from_user, offer });
                            }
                        }
                        // Llamada rechazada
                        ServerMessage::CallRejected { from_user } => {
                            eprintln!("[AppUI] {} rejected the call", from_user);
                            log_c!("[AppUI] {} rejected the call", from_user);
                            self.liberar_recursos();
                            self.cambiar_vista(CambioVista::ToLobby {
                                username: String::new(),
                            });
                        }
                        // Llamada finalizada
                        ServerMessage::CallEnded { from_user } => {
                            eprintln!("[AppUI] {} finished the call", from_user);
                            log_c!("[AppUI] {} finished the call", from_user);
                            self.liberar_recursos();
                            self.crear_camara();
                            self.vista_actual = Vista::VistaCallEnd(VistaCallEnd::new());
                        }
                        // Otros
                        _ => {
                            if let Some(tx) = &self.sdp_handshake_tx {
                                let _ = tx.send(msg);
                            }
                        }
                    }
                }
                AppEvent::ConnectionLost => {
                    self.cambiar_vista(CambioVista::ToLogin);
                    if let Vista::VistaLogin(login) = &mut self.vista_actual {
                        login.error_msg = Some("Conection lost".into());
                    }
                }
                AppEvent::FileSendingRequest {
                    from_user,
                    filename,
                } => {
                    self.cambiar_vista(CambioVista::SendFileRequested {
                        from_user,
                        filename,
                    });
                }
                AppEvent::FileSendingAccepted {
                    from_user,
                    filename,
                } => {
                    // Si el Receptor del archivo aceptó la solicitud mando el FilePathPacket al Client
                    self.tx_file.as_ref().map(|tx| {
                        if let Some(path) = &self.file_path {
                            let _ = tx.send(path.clone());
                        }
                    });
                    self.cambiar_vista(CambioVista::TargetUserSendFileAccepted {
                        from_user,
                        filename,
                    });
                }
                AppEvent::FileSendingRejected {
                    from_user,
                    filename,
                } => {
                    self.cambiar_vista(CambioVista::TargetUserSendFileRejected {
                        from_user,
                        filename,
                    });
                }
            }
        }

        if let Vista::VistaCallStart(_) = &self.vista_actual
            && let Some(rx_result) = &self.pedido_socket
            && let Ok(result) = rx_result.try_recv()
        {
            if let Some(h) = self.handshake_handle.take() {
                let _ = h.join();
            }
            match result {
                Ok((socket, _, _)) => {
                    self.pedido_socket = None;

                    let is_caller = self.pending_offer.is_none();
                    self.crear_cliente(socket, is_caller);

                    self.cambiar_vista(CambioVista::ToCall);
                }
                Err(_) => self.cambiar_vista(CambioVista::ToLobby {
                    username: String::new(),
                }),
            }
        }

        // Call status watcher
        if let Vista::VistaLlamada(_) = &self.vista_actual
            && !self.in_call.load(Ordering::SeqCst)
        {
            self.cambiar_vista(CambioVista::ToCallEnd);
        }

        if let Some(stream) = &mut self.server_writer {
            let cambio = self
                .vista_actual
                .ui(ctx, &self.camara, stream, &self.audio_system);
            self.cambiar_vista(cambio);
        }

        ctx.request_repaint();
    }
}
