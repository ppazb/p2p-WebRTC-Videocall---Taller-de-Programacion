use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::camera::Camera;
use crate::client::Client;
use crate::client_errors::ClientErrors;
use crate::config::{AppParameters, SdpHandshakeParameters};
use crate::ice::{crear_conexion_ice, elegir_codec};
use crate::sdp_errors::SdpError;
use crate::sdp_protocol::sdp_handshake;
use crate::vista::call_end::VistaCallEnd;
use crate::vista::call_start::VistaCallStart;
use crate::vista::lobby::VistaLobby;
use crate::vista::{Vista, cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin};
use eframe::egui;
use opencv::core::Mat;

pub struct AppUI {
    // Disposición de widgets
    vista_actual: Vista,
    camara: Option<Arc<Mutex<Camera>>>,

    // Elementos para conexión entre clientes
    pedido_socket: Option<Receiver<Result<UdpSocket, SdpError>>>,
    handshake_handle: Option<JoinHandle<Result<(), ClientErrors>>>,

    // DEBUG: puertos para cliente
    app_params: AppParameters,

    // Elementos para envío y recepción de datos
    cliente: Option<Client>,
    client_handles: Result<Vec<JoinHandle<Result<(), ClientErrors>>>, ClientErrors>,
    rx_remoto: Option<Receiver<Mat>>, // Canal de recepción para UI
    in_call: Arc<AtomicBool>,
    signaling_comunicator: TcpStream,
}

impl Drop for AppUI {
    fn drop(&mut self) {
        self.liberar_recursos();
    }
}

impl AppUI {
    // Modifico firma AppUI
    pub fn new(app_params: AppParameters) -> Option<Self> {
        if let Ok(stream) = TcpStream::connect(&app_params.sdp_handshake.server_adder) {
            Some(Self {
                vista_actual: Vista::VistaLogin(VistaLogin::new()),
                cliente: None,
                rx_remoto: None,
                in_call: Arc::new(AtomicBool::new(false)),

                pedido_socket: None,
                handshake_handle: None,
                client_handles: Ok(Vec::new()),

                app_params,
                signaling_comunicator: stream,
                camara: None,
            })
        } else {
            None
        }
    }

    fn cambiar_vista(&mut self, cambio: CambioVista) {
        match cambio {
            CambioVista::None => {
                self.observar_cambios();
            }
            CambioVista::ToLogin => {
                // Desde Lobby
                self.liberar_recursos();
                self.vista_actual = Vista::VistaLogin(VistaLogin::new());
            }
            CambioVista::ToLobby => {
                // Desde Login/CallStart/Call

                // Si volví de CallStart, anulo thread de handshake
                self.in_call.store(false, Ordering::Relaxed);
                self.liberar_handshake_handle();

                self.crear_camara();
                self.vista_actual = Vista::VistaLobby(VistaLobby::new());
            }
            CambioVista::ToCallStart => {
                // Desde Lobby
                self.crear_camara();
                self.iniciar_pedido_cliente();
                self.vista_actual = Vista::VistaCallStart(VistaCallStart::new());
            }
            CambioVista::ToCall => {
                // Desde CallStart
                // Envío los receptores a la vista (dejan de estar disponibles en AppUI)
                self.vista_actual = Vista::VistaLlamada(VistaLlamada::new(
                    self.rx_remoto.take(),
                    Arc::clone(&self.in_call),
                ))
            }
            CambioVista::ToCallEnd => {
                // Desde Call
                self.liberar_recursos();

                self.crear_camara();
                self.vista_actual = Vista::VistaCallEnd(VistaCallEnd::new());
            }
            CambioVista::CamEnable => {
                self.iniciar_camara();
            }
            CambioVista::CamDisable => {
                self.detener_camara();
            }
        }
    }

    fn liberar_recursos(&mut self) {
        self.cliente = None;
        self.rx_remoto = None;
        self.in_call.store(false, Ordering::Relaxed);

        self.pedido_socket = None;

        self.liberar_handshake_handle();
        self.liberar_client_handles();
        self.liberar_camara();
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
        // Actualizo flag para controlar vida de threads de conexión
        self.in_call.store(true, Ordering::SeqCst);

        // Copias necesarias para mover al thread

        let handshake_params: SdpHandshakeParameters = self.app_params.sdp_handshake.clone();

        // let my_address = self.app_params.local_adder.clone();
        // let server_address = self.app_params.server_adder.clone();
        // let peer = self.remote_adr.clone();
        let in_call = Arc::clone(&self.in_call); // USAR IN_CALL PARA CANCELAR LA SOLICITUD!!

        // Canal para devolver el nuevo socket
        let (tx_result, rx_result) = std::sync::mpsc::channel();

        let handshake_handle = std::thread::spawn(move || -> Result<(), ClientErrors> {
            //SDP
            let ice_info = match sdp_handshake(in_call, handshake_params) {
                Err(e) => {
                    eprintln!("SDP Handshake error: {}", e);
                    let _ = tx_result.send(Err(e.clone()));
                    return Err(ClientErrors::SdpError(e));
                }
                Ok(info) => info,
            };

            //VALID CODEC? -> if doesnt raise error then ok
            let _codec = match elegir_codec(&ice_info.2) {
                Err(e) => {
                    eprintln!("{}", e);
                    //let _ = tx_result.send(Err(e));
                    return Err(ClientErrors::IceError(e));
                }
                Ok(c) => c,
            };

            //ICE
            let socket_connection = match crear_conexion_ice(&ice_info.0, &ice_info.1) {
                Err(e) => {
                    eprintln!("{}", e);
                    //let _ = tx_result.send(Err(e));
                    return Err(ClientErrors::IceError(e));
                }
                Ok(socket) => socket,
            };

            let _ = tx_result.send(Ok(socket_connection));
            Ok(())
        });

        // Cargo el potencial socket y espero que termine el proceso
        self.pedido_socket = Some(rx_result);
        self.handshake_handle = Some(handshake_handle);
    }

    pub fn crear_cliente(&mut self, socket: UdpSocket) {
        // Creo channel para la recepción de frames remotos, y el envío a la vista
        let (tx_remoto, rx_remoto) = std::sync::mpsc::sync_channel(1);
        self.rx_remoto = Some(rx_remoto);

        // Cargo e inicializo los threads de envío y recepción de frames
        if let Some(cam) = &self.camara {
            let cliente = Client::new(tx_remoto, Arc::clone(&self.in_call), Arc::clone(cam));
            if let Ok(mut c) = cliente {
                let _ = c.add_peer(socket);
                // c.set_rol(true); Esto debemos sacarlo de SDP
                //let _ = c.perform_dtls_handshake(); //No hace nada todavia
                self.client_handles = c.initialize_call();
                self.cliente = Some(c);
            }
        } else {
            eprintln!("CAMARA NO INICIALIZADA!");
        }
    }

    fn observar_cambios(&mut self) {
        match self.vista_actual {
            Vista::VistaCallStart(_) => {
                // Observo el channel para ver si se generó un socket
                if let Some(rx_result) = &self.pedido_socket
                    && let Ok(result) = rx_result.try_recv()
                {
                    // Cierro el thread de handshake ya finalizado
                    if let Some(h) = self.handshake_handle.take() {
                        let _ = h.join();
                    }

                    // Creo el nuevo cliente
                    match result {
                        Ok(s) => {
                            //self.liberar_camara(); // Cierro la cámara para usarla en el thread de envío
                            self.pedido_socket = None;

                            self.crear_cliente(s);
                            self.cambiar_vista(CambioVista::ToCall);
                        }
                        Err(e) => {
                            eprintln!("Error en handshake: {:?}", e);
                            self.cambiar_vista(CambioVista::ToLobby);
                        }
                    }
                }
            }
            Vista::VistaLlamada(_) => {
                if !self.in_call.load(Ordering::SeqCst) {
                    self.cambiar_vista(CambioVista::ToCallEnd);
                }
            }
            _ => {}
        }
    }

    fn liberar_client_handles(&mut self) {
        if let Ok(handles) = &mut self.client_handles {
            while !handles.is_empty() {
                if let Some(handle) = handles.pop() {
                    let _ = handle.join();
                }
            }
        }
    }

    fn liberar_handshake_handle(&mut self) -> Option<UdpSocket> {
        // Reviso si el channel contiene el socket generado
        if let Some(rx_result) = &self.pedido_socket {
            // Cierro el thread de handshake ya finalizado
            if let Some(h) = self.handshake_handle.take() {
                let _ = h.join();
            }

            // Extraigo y devuelvo el socket
            if let Ok(Ok(r)) = rx_result.try_recv() {
                return Some(r);
            }
        }
        None
    }
}

impl eframe::App for AppUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ctx.set_visuals(egui::Visuals::light());

        // cada frame, la vista actual muestra su ui
        // algunas acciones provocan cambios de vista
        // se debería preservar información y cargarla durante la transicion
        let cambio = self
            .vista_actual
            .ui(ctx, &self.camara, &mut self.signaling_comunicator);
        self.cambiar_vista(cambio);
        ctx.request_repaint();
    }
}
