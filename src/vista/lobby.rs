use eframe::egui;

use crate::vista::cambio::CambioVista;

pub struct VistaLobby {
    
}

impl VistaLobby {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Cargando...");
        });

        return cambio;
    }
}