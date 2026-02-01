use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
};

use eframe::egui::{self, TextureHandle};
use rustls::StreamOwned;

use crate::{audio_processing::AudioSystem, camera::Camera, vista::cambio::CambioVista};

pub struct VistaCallEnd {
    preview_texture: Option<TextureHandle>,
}

impl Default for VistaCallEnd {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaCallEnd {
    pub fn new() -> Self {
        Self {
            preview_texture: None,
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        camara: &Option<Arc<Mutex<Camera>>>,
        audio_system: &Arc<AudioSystem>,
        _tls_tcp: &mut Arc<std::sync::Mutex<StreamOwned<rustls::ClientConnection, TcpStream>>>,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::TopBottomPanel::top("menu_bar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.heading("Llamada Finalizada");
                    ui.separator();

                    ui.add_space(ui.available_width() - 100.0);
                });
            });

        if cambio != CambioVista::None {
            return cambio;
        }

        cambio = crate::vista::Vista::camera_preview_side_panel(ctx, camara, audio_system, &mut self.preview_texture);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Centramos verticalmente el contenido restante
            ui.add_space(ui.available_height() / 2.0 - 60.0);

            ui.vertical_centered(|ui| {
                ui.heading("La llamada ha finalizado.");

                ui.add_space(20.0);

                if ui.button("Volver al Lobby").clicked() {
                    // Enviamos string vacío para indicar "Mismo usuario"
                    cambio = CambioVista::ToLobby {
                        username: String::new(),
                    };
                }
            });
        });

        cambio
    }
}
