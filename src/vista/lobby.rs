use crate::{audio_processing::AudioSystem, camera::Camera, vista::cambio::CambioVista};
use eframe::egui::{self, Color32, Frame, RichText, TextureHandle};
use rustls::StreamOwned;
use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
};

pub struct VistaLobby {
    pub users_list: Vec<String>,
    preview_texture: Option<TextureHandle>,
}

impl Default for VistaLobby {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaLobby {
    pub fn new() -> Self {
        Self {
            users_list: Vec::new(),
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
                    ui.heading("Lobby - Usuarios Conectados");
                    ui.separator();

                    ui.add_space(ui.available_width() - 120.0);

                    if ui.button("Cerrar Sesión").clicked() {
                        cambio = CambioVista::ToLogin;
                    }
                });
            });

        if cambio != CambioVista::None {
            return cambio;
        }

        cambio = crate::vista::Vista::camera_preview_side_panel(ctx, camara, audio_system, &mut self.preview_texture);

        egui::CentralPanel::default().show(ctx, |ui| {
            Frame::new()
                .fill(Color32::DARK_GRAY)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.heading(
                        RichText::new("Selecciona un usuario para llamar:").color(Color32::WHITE),
                    );
                    ui.separator();
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height() - 50.0)
                        .show(ui, |ui| {
                            if self.users_list.is_empty() {
                                ui.label(
                                    RichText::new(
                                        "No hay otros usuarios conectados en este momento.",
                                    )
                                    .italics(),
                                );
                            } else {
                                for user in &self.users_list {
                                    // Generamos una fila por usuario
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("👤 {}", user))
                                                .strong()
                                                .size(16.0),
                                        );

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("📞 Llamar".to_string()).clicked() {
                                                    cambio = CambioVista::ToCallStart {
                                                        username: user.clone(),
                                                    };
                                                }
                                            },
                                        );
                                    });
                                    ui.separator();
                                }
                            }
                        });
                });
        });

        cambio
    }
}
