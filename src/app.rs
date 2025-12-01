use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::camera::Camera;
use crate::client::Client;
use crate::client_errors::ClientErrors;
use crate::config::AppParameters;
use crate::ice::{crear_conexion_ice, elegir_codec};
use crate::sdp_errors::SdpError;
use crate::sdp_protocol::sdp_handshake;
use crate::signalling_protocol::{ClientMessage, ServerMessage};
use crate::vista::call_end::VistaCallEnd;
use crate::vista::call_start::VistaCallStart;
use crate::vista::incoming_call::VistaIncomingCall;
use crate::vista::called_user_busy::VistaCalledUserBusy;
use crate::vista::lobby::VistaLobby;
use crate::vista::sign_up::VistaSignUp;
use crate::vista::{Vista, cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin};
use eframe::egui;
use opencv::core::Mat;

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
    CalledUserBusy
}

pub struct AppUI {
    vista_actual: Vista,
    camara: Option<Arc<Mutex<Camera>>>,
    server_events_rx: Receiver<AppEvent>,
    server_writer: Option<TcpStream>,
    sdp_handshake_tx: Option<Sender<ServerMessage>>,
    pedido_socket: Option<Receiver<Result<UdpSocket, SdpError>>>,
    handshake_handle: Option<JoinHandle<Result<(), ClientErrors>>>,
    pending_offer: Option<String>,
    app_params: AppParameters,
    cliente: Option<Client>,
    client_handles: Result<Vec<JoinHandle<Result<(), ClientErrors>>>, ClientErrors>,
    rx_remoto: Option<Receiver<Mat>>,
    in_call: Arc<AtomicBool>,
    current_username: Option<String>,
    target_username: Option<String>,
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
            let _ = writer.write_all(msg.to_json_line().as_bytes());
        }
        self.liberar_recursos();
    }
}

impl AppUI {
    pub fn new(app_params: AppParameters) -> Option<Self> {
        if let Ok(stream) = TcpStream::connect(&app_params.sdp_handshake.server_adder) {
            stream.set_nonblocking(false).ok();
            let read_stream = stream
                .try_clone()
                .expect("Failed to clone stream for reading");
            let write_stream = stream
                .try_clone()
                .expect("Failed to clone stream for writing");
            let (tx, rx) = channel::<AppEvent>();

            // Thread para escuchar del servidor
            std::thread::spawn(move || {
                let mut reader = BufReader::new(read_stream);
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => {
                            let _ = tx.send(AppEvent::ConnectionLost);
                            break;
                        }
                        Ok(_) => {
                            if let Some(msg) = ServerMessage::from_json(&line) {
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
                                    // Fixed: CalledUserBusy is a unit variant; match without `{ .. }`
                                    ServerMessage::CalledUserBusy => {
                                        let _ = tx.send(AppEvent::CalledUserBusy);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(_) => {
                            let _ = tx.send(AppEvent::ConnectionLost);
                            break;
                        }
                    }
                }
            });

            Some(Self {
                //Inicia en vista de login por deffault
                vista_actual: Vista::VistaLogin(VistaLogin::new()),
                server_events_rx: rx,
                server_writer: Some(write_stream),
                sdp_handshake_tx: None,
                cliente: None,
                rx_remoto: None,
                in_call: Arc::new(AtomicBool::new(false)),
                pedido_socket: None,
                handshake_handle: None,
                client_handles: Ok(Vec::new()),
                pending_offer: None,
                app_params,
                camara: None,
                current_username: None,
                target_username: None,
            })
        } else {
            None
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
                    let _ = writer.write_all(msg.to_json_line().as_bytes());
                }
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
                    let _ = writer.write_all(msg.to_json_line().as_bytes());
                }
                self.in_call.store(false, Ordering::Relaxed);
                self.liberar_handshake_handle();
                self.target_username = None;
                self.pending_offer = None;
                self.crear_camara();
                self.vista_actual = Vista::VistaLobby(VistaLobby::new());
            }

            // Pantalla aceptar y rechazar llamda
            CambioVista::ToCallStart { username } => {
                self.target_username = Some(username.clone());
                self.crear_camara();
                self.iniciar_pedido_cliente();

                if let Some(writer) = &mut self.server_writer {
                    let msg = ClientMessage::GetUsers {};
                    let _ = writer.write_all(msg.to_json_line().as_bytes());
                }

                self.vista_actual = Vista::VistaCallStart(VistaCallStart::new(username));
            }

            // paso a la vista de aceptar o rechazar llamada
            CambioVista::ToIncomingCall { caller } => {
                self.target_username = Some(caller.clone());
                self.vista_actual = Vista::VistaIncomingCall(VistaIncomingCall::new(caller));
            }

            CambioVista::CalledUserBusy => {
                eprintln!("[CLIENT] Called user was busy");
                self.vista_actual = Vista::VistaCalledUserBusy(VistaCalledUserBusy::new());
            }

            // acepto la llamada -> nueva vista
            CambioVista::CallAccepted { target_user } => {
                eprintln!("[CLIENT] Called accepted from: {}", target_user);
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
                if let (Some(writer), Some(me)) = (&mut self.server_writer, &self.current_username){
                    let msg = ClientMessage::CallStarted { 
                        username: me.clone(),
                        target_user: target_user.clone()
                    };
                    let _ = writer.write_all(msg.to_json_line().as_bytes());
                }

                self.vista_actual = Vista::VistaCallStart(VistaCallStart::new(target_user));
            }

            // Rechazar llamada -> vuelvo a lobby
            CambioVista::CallRejected { target_user } => {
                eprintln!("[CLIENT] Rejected call from {}", target_user);
                if let (Some(writer), Some(me)) = (&mut self.server_writer, &self.current_username)
                {
                    //hay que comunicarle al otro cliente
                    let msg = ClientMessage::CallReject {
                        username: me.clone(),
                        target_user: target_user.clone(),
                    };
                    let _ = writer.write_all(msg.to_json_line().as_bytes());
                }
                self.pending_offer = None;
                self.cambiar_vista(CambioVista::ToLobby {
                    username: String::new(),
                });
            }

            // Llamada aceptada
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

            // finalizo llamada -> pantalla de fin de llamada con boton a volver al lobby
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
                        let _ = writer.write_all(msg.to_json_line().as_bytes());
                    }
                }
                self.liberar_recursos();
                self.crear_camara();
                self.vista_actual = Vista::VistaCallEnd(VistaCallEnd::new());
            }

            CambioVista::CamEnable => self.iniciar_camara(),
            CambioVista::CamDisable => self.detener_camara(),
        }
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
            let mut writer_clone = writer.try_clone().expect("Failed to clone writer");
            let (tx_result, rx_result) = std::sync::mpsc::channel();

            let handshake_handle = std::thread::spawn(move || -> Result<(), ClientErrors> {
                eprintln!("[DEBUG THREAD] Starting sdp_handshake...");
                let ice_info = match sdp_handshake(
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
                let _codec = match elegir_codec(&ice_info.2) {
                    Err(e) => return Err(ClientErrors::IceError(e)),
                    Ok(c) => c,
                };
                let socket = match crear_conexion_ice(&ice_info.0, &ice_info.1) {
                    Err(e) => return Err(ClientErrors::IceError(e)),
                    Ok(s) => s,
                };
                let _ = tx_result.send(Ok(socket));
                Ok(())
            });
            self.pedido_socket = Some(rx_result);
            self.handshake_handle = Some(handshake_handle);
        }
    }

    fn crear_cliente(&mut self, socket: UdpSocket) {
        let (tx_remoto, rx_remoto) = std::sync::mpsc::sync_channel(1);
        self.rx_remoto = Some(rx_remoto);
        if let Some(cam) = &self.camara {
            let cliente = Client::new(tx_remoto, Arc::clone(&self.in_call), Arc::clone(cam));
            if let Ok(mut c) = cliente {
                let _ = c.add_peer(socket);
                self.client_handles = c.initialize_call();
                self.cliente = Some(c);
            }
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
                        // Vacío los campos y notifico el error
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
                            self.liberar_recursos();
                            self.cambiar_vista(CambioVista::ToLobby {
                                username: String::new(),
                            });
                        }
                        // Llamada finalizada
                        ServerMessage::CallEnded { from_user } => {
                            eprintln!("[AppUI] {} finished the call", from_user);
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
                        login.error_msg = Some("Se perdió la conexión al servidor.".into());
                    }
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
                Ok(s) => {
                    self.pedido_socket = None;
                    self.crear_cliente(s);
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
            let cambio = self.vista_actual.ui(ctx, &self.camara, stream);
            self.cambiar_vista(cambio);
        }

        ctx.request_repaint();
    }
}
