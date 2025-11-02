use eframe::egui;

use crate::vista::cambio::CambioVista;

pub struct VistaCallEnd {
    
}

impl VistaCallEnd {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("La llamada ha finalizado.");

            if ui.button("Volver al Lobby").clicked() {
                cambio = CambioVista::ToLobby;
            }
        });

        return cambio;
    }
}