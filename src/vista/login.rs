use crate::signalling_protocol::ClientMessage;
use crate::vista::cambio::CambioVista;
use eframe::egui::{self, Color32, Frame, Label, RichText, Sense, TextEdit, Vec2};
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
                                    RichText::new("Login")
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
                                        TextEdit::singleline(&mut self.username),
                                    );
                                });

                                ui.add_space(10.0);

                                // Campo contraseña
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Contraseña:").color(Color32::WHITE));
                                    ui.add_sized(
                                        //Vec2::new(160.0, 22.0),
                                        Vec2::new(ui.available_width(), 22.0),
                                        TextEdit::singleline(&mut self.password).password(true),
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

                                // Botón de ingreso
                                if self.auth_pending {
                                    ui.spinner();
                                    ui.label(
                                        RichText::new("Conectando al servidor...")
                                            .color(Color32::LIGHT_GRAY),
                                    );
                                } else {
                                    if ui.button("Iniciar Sesión").clicked() {
                                        if self.username.is_empty() || self.password.is_empty() {
                                            self.error_msg = Some("Complete todos los campos.".into());
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

                                    // Botón de registro
                                    ui.add_space(10.0);
                                    let label = ui.add(Label::new("Crear nueva cuenta").selectable(false).sense(Sense::click()));
                                    if label.clicked() {
                                        cambio = CambioVista::ToSignUp;
                                    }  
                                }                          
                            });
                        });
                }
            );
        });

        // Con excepción de la operación de registro, el cambio se lo comunica el server a AppUI desde fuera
        cambio
    }
}
