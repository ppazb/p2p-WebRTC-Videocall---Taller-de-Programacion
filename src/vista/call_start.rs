use eframe::egui;

use crate::{camera::Camera, vista::cambio::CambioVista};

pub struct VistaCallStart {
    
}

impl VistaCallStart {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Camera>) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::TopBottomPanel::top("menu_bar").exact_height(50.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("Llamada en Curso");
                ui.separator();
                
                ui.add_space(ui.available_width()-100.0);

            });
        });

        cambio = crate::vista::Vista::camera_preview_side_panel(ctx, camara);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Iniciando llamada...");

            ui.add_space(20.0);

            if ui.button("Volver al Lobby").clicked() {
                cambio = CambioVista::ToLobby;
            }
        });

        return cambio;
    }
}