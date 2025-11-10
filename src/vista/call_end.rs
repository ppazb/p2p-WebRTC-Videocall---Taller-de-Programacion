use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::{camera::Camera, vista::cambio::CambioVista};

pub struct VistaCallEnd {}

impl Default for VistaCallEnd {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaCallEnd {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Arc<Mutex<Camera>>>) -> CambioVista {
        let mut cambio;

        egui::TopBottomPanel::top("menu_bar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.heading("Llamada en Curso");
                    ui.separator();

                    ui.add_space(ui.available_width() - 100.0);
                });
            });

        cambio = crate::vista::Vista::camera_preview_side_panel(ctx, camara);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(ui.available_height() / 2.0 - 40.0);

            ui.vertical_centered(|ui| {
                ui.heading("La llamada ha finalizado.");

                ui.add_space(20.0);

                if ui.button("Volver al Lobby").clicked() {
                    cambio = CambioVista::ToLobby;
                }
            });
        });

        cambio
    }
}
