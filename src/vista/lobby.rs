use eframe::egui;

use crate::vista::cambio::CambioVista;

pub struct VistaLobby {}

impl Default for VistaLobby {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaLobby {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista {
        let cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Cargando...");
        });

        cambio
    }
}
