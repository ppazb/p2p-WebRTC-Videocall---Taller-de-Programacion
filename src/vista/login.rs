use std::{io::{Read, Write}, str::FromStr, sync::mpsc};

use eframe::egui::{self, Color32, Frame, RichText, TextEdit, Vec2};

use crate::vista::cambio::CambioVista;

pub struct VistaLogin {
    pub username: String,
    pub password: String,
    pub error_msg: Option<String>,
    pub auth_rx: Option<mpsc::Receiver<String>>,
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
            auth_rx: None,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, tcp: &mut std::net::TcpStream) -> CambioVista {
        let mut cambio = CambioVista::None;

        // Procesar resultado de autenticación anterior si existe
        if let Some(rx) = &self.auth_rx {
            match rx.try_recv() {
                Ok(response) => {
                    eprintln!("[DEBUG] Auth response received: {}", response);
                    self.auth_rx = None;
                    if response.contains("OK") || response.contains("ok") {
                        cambio = CambioVista::ToLobby;
                    } else {
                        self.error_msg = Some("Usuario o contraseña inválidos.".into());
                        self.password.clear();
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.error_msg = Some("Verificando credenciales...".into());
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.auth_rx = None;
                    self.error_msg = Some("Error de conexión con el servidor.".into());
                }
            }
        }

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
                                    if ui.button("Iniciar Sesión").clicked() && self.auth_rx.is_none() {
                                        let user = String::from(&self.username);
                                        let user_msg = String::from(user + "\n");
                                        let pass = String::from(&self.password);
                                        let pass_msg = String::from(pass + "\n");

                                        let mut tcp_signalling = tcp.try_clone().unwrap();
                                        //tcp_signalling.set_read_timeout(Some(std::time::Duration::from_secs(5)))
                                        //    .expect("Failed to set read timeout");
                                        let (tx, rx) = mpsc::channel::<String>();
                                        self.auth_rx = Some(rx);

                                        std::thread::spawn(move || {
                                            use std::io::BufRead;
                                            
                                            eprintln!("[DEBUG CLIENT] Sending user: {}", user_msg.trim());
                                            if let Err(e) = tcp_signalling.write_all(user_msg.as_bytes()) {
                                                eprintln!("[ERROR] Failed to write username: {}", e);
                                                let _ = tx.send(format!("ERROR: {}", e));
                                                return;
                                            }
                                            
                                            eprintln!("[DEBUG CLIENT] Sending pass: {}", pass_msg.trim());
                                            if let Err(e) = tcp_signalling.write_all(pass_msg.as_bytes()) {
                                                eprintln!("[ERROR] Failed to write password: {}", e);
                                                let _ = tx.send(format!("ERROR: {}", e));
                                                return;
                                            }
                                            
                                            if let Err(e) = tcp_signalling.flush() {
                                                eprintln!("[ERROR] Failed to flush: {}", e);
                                                let _ = tx.send(format!("ERROR: {}", e));
                                                return;
                                            }

                                            eprintln!("[DEBUG CLIENT] Waiting for response...");
                                            let mut reader = std::io::BufReader::new(&tcp_signalling);
                                            let mut response = String::new();
                                            match reader.read_line(&mut response) {
                                                Ok(n) if n > 0 => {
                                                    let resp_trimmed = response.trim().to_string();
                                                    eprintln!("[DEBUG CLIENT] Received response: {}", resp_trimmed);
                                                    let _ = tx.send(resp_trimmed);
                                                }
                                                Ok(_) => {
                                                    eprintln!("[ERROR] Connection closed by server");
                                                    let _ = tx.send("ERROR: Connection closed".to_string());
                                                }
                                                Err(e) => {
                                                    eprintln!("[ERROR] Failed to read response: {}", e);
                                                    let _ = tx.send(format!("ERROR: {}", e));
                                                }
                                            }
                                        });
                                        
                                        self.error_msg = Some("Enviando credenciales...".into());
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
