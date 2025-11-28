use crate::signalling_protocol::{ClientMessage, write_all_nonblocking};
use crate::vista::cambio::CambioVista;
use eframe::egui::{self, Color32, Frame, RichText, TextEdit, Vec2};
use rustls::StreamOwned;
use std::{
    io::Write,
    net::TcpStream,
    sync::{Arc, atomic::AtomicBool},
};

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

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        tls_tcp: &mut Arc<std::sync::Mutex<StreamOwned<rustls::ClientConnection, TcpStream>>>,
    ) -> CambioVista {
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

                                        self.auth_pending = true;
                                        self.error_msg = None;

                                        let tls = Arc::clone(tls_tcp);
                                        std::thread::spawn(move || {
                                                // este código ya NO toca self, solo el TLS
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
                                                    // acá ya no podemos tocar self; solo logueamos
                                                }
                                                guard_write.flush();
                                                // guard_write se suelta acá al salir del scope
                                        });

                                        // match tcp.write_all(json_msg.as_bytes()) {
                                        //     Ok(_) => {
                                        //         let _ = tcp.flush();
                                        //         self.auth_pending = true;
                                        //         self.error_msg = None;
                                        //     }
                                        //     Err(e) => {
                                        //         self.error_msg =
                                        //             Some(format!("Error de red: {}", e));
                                        //         eprintln!("[ERROR] Write failed: {}", e);
                                        //     }
                                        // }
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
