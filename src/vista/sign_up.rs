use std::{io::Write, net::TcpStream, sync::Arc};

use eframe::egui::{self, Color32, Frame, Label, RichText, Sense, TextEdit, Vec2};
use rustls::StreamOwned;

use crate::{
    signalling_protocol::{ClientMessage, write_all_nonblocking},
    vista::cambio::CambioVista,
};

pub struct VistaSignUp {
    pub username: String,
    pub password: String,
    pub error_msg: Option<String>,
    pub auth_pending: bool,
}

impl VistaSignUp {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            error_msg: None,
            auth_pending: false,
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        tls_tcp: &mut Arc<std::sync::Mutex<StreamOwned<rustls::ClientConnection, TcpStream>>>,
    ) -> CambioVista {
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
                            ui.set_width(260.0); // 240
                            ui.set_height(250.0); //150

                            ui.vertical_centered(|ui| {
                                ui.heading(
                                    RichText::new("Sign Up")
                                        .color(Color32::WHITE)
                                        .strong(),
                                );
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(10.0);

                                // Campo usuario
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Usuario:").color(Color32::WHITE));
                                    ui.add_sized(
                                        Vec2::new(ui.available_width(), 22.0),
                                        TextEdit::singleline(&mut self.username).hint_text("5-32 caracteres alfanuméricos"),
                                    );
                                });

                                ui.add_space(10.0);

                                // Campo contraseña
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Contraseña:").color(Color32::WHITE));
                                    ui.add_sized(
                                        //Vec2::new(160.0, 22.0),
                                        Vec2::new(ui.available_width(), 22.0),
                                        TextEdit::singleline(&mut self.password).password(true).hint_text("5-32 caracteres"),
                                    );
                                });

                                ui.add_space(20.0);

                                // Aviso de datos ingresados
                                if let Some(msg) = &self.error_msg {
                                    ui.colored_label(egui::Color32::LIGHT_RED, msg);
                                } else {
                                    ui.label("");
                                }

                                ui.add_space(10.0);

                                // Botón de Registro
                                if self.auth_pending {
                                    ui.spinner();
                                    ui.label(
                                        RichText::new("Conectando al servidor...")
                                            .color(Color32::LIGHT_GRAY),
                                    );
                                } else {
                                    if ui.button("Registrar Usuario").clicked() {
                                        if self.username.is_empty() || self.password.is_empty() {
                                            self.error_msg = Some("Complete todos los campos.".into());
                                        } else {
                                            // Crear mensaje a partir del contenido de los campos
                                            let signup_msg = ClientMessage::SignUp {
                                                username: self.username.clone(),
                                                password: self.password.clone(),
                                            };
                                            let json_msg = signup_msg.to_json_line();

                                            eprintln!(
                                                "[CLIENT] Enviando signup para: {}",
                                                self.username
                                            );

                                            // Envío el mensaje al server por TCP
                                            self.auth_pending = true;
                                            self.error_msg = None;

                                            let tls = Arc::clone(tls_tcp);
                                            std::thread::spawn(move || {
                                                let mut guard_write = match tls.lock() {
                                                    Ok(gw) => {
                                                        eprintln!(
                                                            "[CLIENT DEBUG] Mutex lock OK, voy a escribir {} bytes",
                                                            json_msg.len()
                                                        );
                                                        gw
                                                    }
                                                    Err(e) => {
                                                        eprintln!("[CLIENT DEBUG] Mutex lock ERROR (poisoned): {e}");
                                                        return;
                                                    }
                                                };
                                                if let Err(e) = write_all_nonblocking(&mut *guard_write, json_msg.as_bytes()) {
                                                    eprintln!("[CLIENT] Error de red al enviar GetUsers: {e}");
                                                }
                                                let _ = guard_write.flush();
                                            });
                                        }
                                    }

                                    // Botón de login
                                    ui.add_space(10.0);
                                    let label = ui.add(Label::new("Ya tengo una cuenta").selectable(false).sense(Sense::click()));
                                    if label.clicked() {
                                        cambio = CambioVista::ToLogin;
                                    }
                                }
                            });
                        });
                }
            );
        });

        return cambio;
    }
}
