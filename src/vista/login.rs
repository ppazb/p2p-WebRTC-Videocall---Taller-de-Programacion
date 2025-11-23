use crate::signalling_protocol::ClientMessage;
use crate::vista::cambio::CambioVista;
use eframe::egui::{self, Color32, Frame, RichText, TextEdit, Vec2};
use std::io::Write;

pub struct VistaLogin {
    pub username: String,
    pub password: String,
    pub error_msg: Option<String>,
    pub auth_pending: bool,
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
            auth_pending: false,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, tcp: &mut std::net::TcpStream) -> CambioVista {
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
                            ui.set_width(260.0);
                            ui.set_height(220.0);

                            ui.vertical_centered(|ui| {
                                ui.heading(
                                    RichText::new("RoomRTC Login")
                                        .color(Color32::WHITE)
                                        .strong(),
                                );
                                ui.add_space(20.0);

                                // Campo usuario
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Usuario:").color(Color32::WHITE));
                                    ui.add_sized(
                                        Vec2::new(160.0, 22.0),
                                        TextEdit::singleline(&mut self.username),
                                    );
                                });

                                ui.add_space(10.0);

                                // Campo contraseña
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Password:").color(Color32::WHITE));
                                    ui.add_sized(
                                        Vec2::new(160.0, 22.0),
                                        TextEdit::singleline(&mut self.password).password(true),
                                    );
                                });

                                ui.add_space(20.0);

                                if let Some(msg) = &self.error_msg {
                                    ui.colored_label(egui::Color32::LIGHT_RED, msg);
                                } else {
                                    ui.label("");
                                }

                                ui.add_space(10.0);

                                if self.auth_pending {
                                    ui.spinner();
                                    ui.label(
                                        RichText::new("Conectando al servidor...")
                                            .color(Color32::LIGHT_GRAY),
                                    );
                                } else if ui.button("Iniciar Sesion").clicked() {
                                    if self.username.is_empty() || self.password.is_empty() {
                                        self.error_msg = Some("Complete todos los campos".into());
                                    } else {
                                        // Crear mensaje
                                        let login_msg = ClientMessage::Login {
                                            username: self.username.clone(),
                                            password: self.password.clone(),
                                        };
                                        let json_msg = login_msg.to_json_line();

                                        eprintln!(
                                            "[CLIENT] Enviando login para: {}",
                                            self.username
                                        );

                                        match tcp.write_all(json_msg.as_bytes()) {
                                            Ok(_) => {
                                                let _ = tcp.flush();
                                                self.auth_pending = true;
                                                self.error_msg = None;
                                            }
                                            Err(e) => {
                                                self.error_msg =
                                                    Some(format!("Error de red: {}", e));
                                                eprintln!("[ERROR] Write failed: {}", e);
                                            }
                                        }
                                    }
                                }
                            });
                        });
                },
            );
        });

        // Siempre retornamos None porque el cambio lo fuerza AppUI desde fuera
        CambioVista::None
    }
}
