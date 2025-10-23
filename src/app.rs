use std::sync::mpsc::Receiver;
use std::thread;

use eframe::egui;
use opencv::core::Mat;
use crate::vista::{cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin, Vista};
use crate::client::{self, Client};

pub struct AppUI {
    vista_actual: Vista,
    cliente: Option<Client>,
    rx_local: Option<Receiver<Mat>>,
    rx_remoto: Option<Receiver<Mat>>
}

impl AppUI {
    pub fn new() -> Self {
        Self {
            vista_actual: Vista::VistaLogin(VistaLogin::new()),
            cliente: None,
            rx_local: None,
            rx_remoto: None
        }
    }

    pub fn crear_cliente(&mut self) {
        // Creo channels para la transmisión y recepción de frames, de ambas cámaras
        let (tx_local, rx_local) = std::sync::mpsc::channel();
        let (tx_remoto, rx_remoto) = std::sync::mpsc::channel();

        // Los receptores son atributos de la app, para poder usarlo en distintas vistas
        self.rx_local = Some(rx_local);
        self.rx_remoto = Some(rx_remoto);

        // Defino puertos e instancio el cliente
        let my_address = "127.0.0.1:4000".into();
        let peer = "127.0.0.1:4000".into();
        
        if let Ok(c) = Client::new(my_address, peer, tx_local, tx_remoto) {
            self.cliente = Some(c);
        }
    }
}

impl eframe::App for AppUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::light()); // o Visuals::light()

        // cada frame, la vista actual muestra su ui
        // algunas acciones provocan cambios de vista
        // se debería preservar información y cargarla durante la transicion
        let cambio = self.vista_actual.ui(ctx);
        
        match cambio {
            CambioVista::Login => self.vista_actual = Vista::VistaLogin(VistaLogin::new()),
            CambioVista::None => {},
            CambioVista::Llamada => {
                // A partir del cambio a la vista llamada, tiene sentido tener un cliente
                self.crear_cliente();

                // thread::spawn({
                //     let cliente = self.cliente.clone();
                //     move || {
                //         if let Err(e) = cliente.initialize_call() {
                //             eprintln!("Error initializing call: {:?}", e);
                //         }
                //     }
                // });
                if let Some(c) = &self.cliente {
                    let _ = c.initialize_call();
                } 

                // Envío los receptores a la vista (dejan de estar disponibles en AppUI)
                self.vista_actual = Vista::VistaLlamada(VistaLlamada::new(self.rx_local.take(), self.rx_remoto.take()))
            },
        }
    }
}
