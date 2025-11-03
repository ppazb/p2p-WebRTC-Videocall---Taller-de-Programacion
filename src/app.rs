use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use crate::camera::Camera;
use crate::client::Client;
use crate::client_errors::ClientErrors;
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
    camara: Option<Camera>,

    // Elementos para conexión entre clientes
    pedido_socket: Option<Receiver<Result<UdpSocket, SdpError>>>,
    handshake_handle: Option<JoinHandle<Result<(), ClientErrors>>>,

    // DEBUG: puertos para cliente
    local_adr: String,
    remote_adr: String,

    // Elementos para envío y recepción de datos
    cliente: Option<Client>,
    client_handles: Result<Vec<JoinHandle<Result<(), ClientErrors>>>, ClientErrors>,
    rx_local: Option<Receiver<Mat>>,
    rx_remoto: Option<Receiver<Mat>>,
    in_call: Arc<AtomicBool>,
}

impl Drop for AppUI {
    fn drop(&mut self) {
        self.liberar_recursos();
    }
}

impl AppUI {
    pub fn new(local_adr: String, remote_adr: String) -> Self {
        Self {
            vista_actual: Vista::VistaLogin(VistaLogin::new()),
            cliente: None,
            rx_local: None,
            rx_remoto: None,
            in_call: Arc::new(AtomicBool::new(false)),

            pedido_socket: None,
            handshake_handle: None,
            client_handles: Ok(Vec::new()),

            local_adr,
            remote_adr,

            camara: None,
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
                    self.rx_local.take(),
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
        self.rx_local = None;
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

        self.camara = Some(camara);
    }

    fn iniciar_camara(&mut self) {
        if let Some(cam) = &mut self.camara {
            cam.enable_camera();
        }
    }

    fn detener_camara(&mut self) {
        if let Some(cam) = &mut self.camara {
            cam.disable_camera();
        }
    }

    fn liberar_camara(&mut self) {
        self.camara = None;
    }

    fn iniciar_pedido_cliente(&mut self) {
        // Actualizo flag para controlar vida de threads de conexión
        self.in_call.store(true, Ordering::SeqCst);

        // Copias necesarias para mover al thread
        let my_address = self.local_adr.clone();
        let peer = self.remote_adr.clone();
        let in_call = Arc::clone(&self.in_call); // USAR IN_CALL PARA CANCELAR LA SOLICITUD!!

        // Canal para devolver el nuevo socket
        let (tx_result, rx_result) = std::sync::mpsc::channel();

        let handshake_handle = std::thread::spawn(move || -> Result<(), ClientErrors> {
            //SDP
            let ice_info = match sdp_handshake(in_call, my_address.as_str(), peer.as_str()) {
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
        // Creo channels para la transmisión y recepción de frames, de ambas cámaras
        let (tx_local, rx_local) = std::sync::mpsc::channel();
        let (tx_remoto, rx_remoto) = std::sync::mpsc::channel();

        // Los receptores son atributos de la app, usados durante la llamada
        self.rx_local = Some(rx_local);
        self.rx_remoto = Some(rx_remoto);

        // Cargo e inicializo los threads de envío y recepción de frames
        let cliente = Client::new(socket, tx_local, tx_remoto, Arc::clone(&self.in_call));
        if let Ok(c) = cliente {
            self.client_handles = c.initialize_call();
            self.cliente = Some(c);
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
                            self.liberar_camara(); // Cierro la cámara para usarla en el thread de envío
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
        let cambio = self.vista_actual.ui(ctx, &self.camara);
        self.cambiar_vista(cambio);
        ctx.request_repaint();
    }
}
