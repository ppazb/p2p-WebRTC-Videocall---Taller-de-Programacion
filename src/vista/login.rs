use eframe::egui::{self, Color32, Frame, RichText, TextEdit, Vec2};

use crate::vista::cambio::CambioVista;

pub struct VistaLogin {
    pub username: String,
    pub password: String,
    pub error_msg: Option<String>,
}

impl Default for VistaLogin {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaLogin {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            error_msg: None,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context,tcp: Option<&mut std::net::TcpStream>) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            // Centro el frame dentro del panel
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    Frame::new()
                        .fill(Color32::DARK_GRAY)
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::same(20))
                        .show(ui, |ui| {
                            ui.set_width(240.0);
                            ui.set_height(150.0);

                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    // Campo usuario
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new("Usuario:").color(Color32::WHITE));
                                        ui.add_sized(
                                            Vec2::new(ui.available_width(), 22.0),
                                            TextEdit::singleline(&mut self.username),
                                        );
                                    });

                                    ui.add_space(20.0);

                                    // Campo contraseña
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new("Contraseña:").color(Color32::WHITE),
                                        );
                                        ui.add_sized(
                                            Vec2::new(ui.available_width(), 22.0),
                                            TextEdit::singleline(&mut self.password).password(true),
                                        );
                                    });

                                    ui.add_space(10.0);

                                    // Aviso de datos ingresados
                                    if let Some(msg) = &self.error_msg {
                                        ui.colored_label(egui::Color32::DARK_RED, msg);
                                    } else {
                                        ui.colored_label(egui::Color32::DARK_RED, "");
                                    }

                                    ui.add_space(10.0);

                                    // Botón de ingreso
                                    if ui.button("Iniciar Sesión").clicked() {
                                        
                                        if self.username.is_empty() && self.password.is_empty() {
                                            cambio = CambioVista::ToLobby;
                                        } else {
                                            self.username = String::new();
                                            self.password = String::new();
                                            self.error_msg =
                                                Some("Usuario o contraseña inválidos.".into());
                                        }
                                    }
                                },
                            );
                        });
                },
            );
        });
        cambio
    }
}
