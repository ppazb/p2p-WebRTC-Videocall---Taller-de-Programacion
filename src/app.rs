use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;

use crate::client::Client;
use crate::client_errors::ClientErrors;
use crate::ice::{crear_conexion_ice, elegir_codec};
use crate::sdp_errors::SdpError;
use crate::sdp_protocol::sdp_handshake;
use crate::vista::lobby::VistaLobby;
use crate::vista::{Vista, cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin};
use eframe::egui;
use opencv::core::Mat;

pub struct AppUI {
    vista_actual: Vista,
    cliente: Option<Client>,
    rx_local: Option<Receiver<Mat>>,
    rx_remoto: Option<Receiver<Mat>>,
    in_call: Arc<AtomicBool>,

    // Se necesita un objeto global de error?
    pedido_cliente: Option<Receiver<Result<Client, SdpError>>>,

    // DEBUG: puertos para cliente
    local_adr: String,
    remote_adr: String,
}

impl AppUI {
    pub fn new(local_adr: String, remote_adr: String) -> Self {
        Self {
            vista_actual: Vista::VistaLogin(VistaLogin::new()),
            cliente: None,
            rx_local: None,
            rx_remoto: None,
            in_call: Arc::new(AtomicBool::new(false)),

            pedido_cliente: None,

            local_adr,
            remote_adr,
        }
    }

    pub fn crear_cliente(&mut self) {
        // Creo channels para la transmisión y recepción de frames, de ambas cámaras
        let (tx_local, rx_local) = std::sync::mpsc::channel();
        let (tx_remoto, rx_remoto) = std::sync::mpsc::channel();

        // Los receptores son atributos de la app, para poder usarlo en distintas vistas
        self.rx_local = Some(rx_local);
        self.rx_remoto = Some(rx_remoto);

        // Copias necesarias para mover al thread
        let my_address = self.local_adr.clone();
        let peer = self.remote_adr.clone();
        let in_call = Arc::clone(&self.in_call);

        // Canal para devolver el resultado del cliente
        let (tx_result, rx_result) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            //SDP
            let ice_info = match sdp_handshake(my_address.as_str(), peer.as_str()) {
                Err(e) => {
                    eprintln!("SDP Handshake error: {}", e);
                    let _ = tx_result.send(Err(e));
                    return;
                }
                Ok(info) => info,
            };

            //VALID CODEC? -> if doesnt raise error then ok
            let _codec = match elegir_codec(&ice_info.2) {
                Err(e) => {
                    eprintln!("{}", e);
                    //let _ = tx_result.send(Err(e));
                    return;
                }
                Ok(c) => c,
            };

            //ICE
            let socket_connection = match crear_conexion_ice(&ice_info.0, &ice_info.1) {
                Err(e) => {
                    eprintln!("{}", e);
                    //let _ = tx_result.send(Err(e));
                    return;
                }
                Ok(socket) => socket,
            };

            // Crear cliente
            match Client::new(socket_connection, tx_local, tx_remoto, in_call) {
                Ok(c) => {
                    let _ = tx_result.send(Ok(c));
                }
                Err(e) => {
                    eprintln!("Error creando cliente: {:?}", e);
                    //let _ = tx_result.send(Err(e));
                    return;
                }
            }
        });

        //SDP
        // let ice_info = match sdp_handshake(my_address, peer) {
        //     Err(e) => {
        //         eprintln!("SDP Handshake error: {}", e);
        //         return;
        //     }
        //     Ok(info) => info,
        // };

        // //VALID CODEC? -> if doesnt raise error then ok
        // let _codec = match elegir_codec(&ice_info.2) {
        //     Err(e) => {
        //         eprintln!("{}", e);
        //         return;
        //     }
        //     Ok(c) => c,
        // };

        // //ICE
        // let socket_connection = match crear_conexion_ice(&ice_info.0, &ice_info.1) {
        //     Err(e) => {
        //         eprintln!("{}", e);
        //         return;
        //     }
        //     Ok(socket) => socket,
        // };

        // if let Ok(c) = Client::new(
        //     socket_connection,
        //     tx_local,
        //     tx_remoto,
        //     Arc::clone(&self.in_call),
        // ) {
        //     self.cliente = Some(c);
        // }

        // Cargo el potencial cliente y espero que termine el proceso
        self.pedido_cliente = Some(rx_result);
    }
}

impl eframe::App for AppUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ctx.set_visuals(egui::Visuals::light());

        // cada frame, la vista actual muestra su ui
        // algunas acciones provocan cambios de vista
        // se debería preservar información y cargarla durante la transicion
        let cambio = self.vista_actual.ui(ctx);

        match cambio {
            CambioVista::Login => {
                // Reinicio atributos de conexión
                self.cliente = None;
                self.rx_local = None;
                self.rx_remoto = None;

                self.vista_actual = Vista::VistaLogin(VistaLogin::new())
            },
            CambioVista::Lobby => {
                // Voy a pantalla de carga, inicializo el cliente
                self.vista_actual = Vista::VistaLobby(VistaLobby::new());

                self.crear_cliente();
            }
            CambioVista::None => {
                match self.vista_actual {
                    // Finalización de llamada, iniciado por el otro cliente
                    Vista::VistaLlamada(_) => {
                        if !self.in_call.load(Ordering::SeqCst) {
                            self.cliente = None;
                            self.rx_local = None;
                            self.rx_remoto = None;
                            
                            self.vista_actual = Vista::VistaLogin(VistaLogin::new())
                        }
                    },
                    // Espero la creación del cliente, cuando se cree paso a la llamada
                    Vista::VistaLobby(_) => {
                        if let Some(rx_result) = &self.pedido_cliente {
                            if let Ok(result) = rx_result.try_recv() {
                                match result {
                                    Ok(cliente) => {
                                        // Se carga el nuevo cliente, debería haber un join para limpiar el thread terminado?
                                        self.cliente = Some(cliente);
                                        self.pedido_cliente = None;
                                        println!("Nuevo cliente creado");

                                        // Inicializo threads de captura+envío y recepción de frames
                                        if let Some(c) = &self.cliente {
                                            let _ = c.initialize_call();
                                        }

                                        // Actualizo flag para controlar vida de threads
                                        self.in_call.store(true, Ordering::SeqCst);

                                        // Envío los receptores a la vista (dejan de estar disponibles en AppUI)
                                        self.vista_actual = Vista::VistaLlamada(VistaLlamada::new(
                                            self.rx_local.take(),
                                            self.rx_remoto.take(),
                                            Arc::clone(&self.in_call),
                                        ))
                                    }
                                    Err(e) => {
                                        eprintln!("Error en handshake: {:?}", e);
                                        self.pedido_cliente = None;
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
            CambioVista::Llamada => {
                // A partir del cambio a la vista llamada, tiene sentido tener un cliente
                // self.crear_cliente();

                // Inicializo threads de captura+envío y recepción de frames
                if let Some(c) = &self.cliente {
                    let _ = c.initialize_call();
                }

                // Actualizo flag para controlar vida de threads
                self.in_call.store(true, Ordering::SeqCst);

                // Envío los receptores a la vista (dejan de estar disponibles en AppUI)
                self.vista_actual = Vista::VistaLlamada(VistaLlamada::new(
                    self.rx_local.take(),
                    self.rx_remoto.take(),
                    Arc::clone(&self.in_call),
                ))
            }
        }
    }
}
