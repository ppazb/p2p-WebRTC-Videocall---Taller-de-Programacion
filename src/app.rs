use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use eframe::egui;
use opencv::core::Mat;
use crate::vista::{cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin, Vista};
use crate::client::Client;

pub struct AppUI {
    vista_actual: Vista,
    cliente: Option<Client>,
    rx_local: Option<Receiver<Mat>>,
    rx_remoto: Option<Receiver<Mat>>,
    in_call: Arc<AtomicBool>
}

impl AppUI {
    pub fn new() -> Self {
        Self {
            vista_actual: Vista::VistaLogin(VistaLogin::new()),
            cliente: None,
            rx_local: None,
            rx_remoto: None,
            in_call: Arc::new(AtomicBool::new(false))
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
        
        if let Ok(c) = Client::new(my_address, peer, tx_local, tx_remoto, Arc::clone(&self.in_call)) {
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
            CambioVista::Login => {
                self.cliente = None;
                self.vista_actual = Vista::VistaLogin(VistaLogin::new())
            },
            CambioVista::None => {},
            CambioVista::Llamada => {
                // A partir del cambio a la vista llamada, tiene sentido tener un cliente
                self.crear_cliente();


                if let Some(c) = &self.cliente {
                    let _ = c.initialize_call();
                }

                self.in_call.store(true, Ordering::Relaxed);

                // Envío los receptores a la vista (dejan de estar disponibles en AppUI)
                self.vista_actual = Vista::VistaLlamada(VistaLlamada::new(self.rx_local.take(), self.rx_remoto.take(), Arc::clone(&self.in_call)))
            },
        }
    }
}
