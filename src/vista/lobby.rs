use crate::{
    camera::Camera,
    signalling_protocol::{ClientMessage, ServerMessage},
    vista::cambio::CambioVista,
};
use eframe::egui::{self, Align, Color32, Frame, Layout, RichText};
use std::{
    io::{BufRead, Write},
    sync::{Arc, Mutex},
};

pub struct VistaLobby {
    //no_camera_img: Option<ColorImage>,
}

impl Default for VistaLobby {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaLobby {
    pub fn new() -> Self {
        Self {
            //no_camera_img: crate::vista::Vista::no_camera_texture(),
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        camara: &Option<Arc<Mutex<Camera>>>,
        tcp: &mut std::net::TcpStream,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;
        let mut tcp_signalling = tcp.try_clone().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            let msg = ClientMessage::GetUsers {};
            let json_msg = msg.to_json_line();
            //     Ok(msg) => msg,
            //     Err(e) => {
            //         eprintln!("[ERROR] Failed to serialize login message: {}", e);
            //         let _ = tx.send(format!("ERROR: {}", e));
            //         return;
            //     }
            // };
            eprintln!("[DEBUG CLIENT] request users message: {}", json_msg.trim());
            if let Err(e) = tcp_signalling.write_all(json_msg.as_bytes()) {
                eprintln!("[ERROR] Failed to write login message: {}", e);
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
                    let answer = response.trim().to_string();
                    eprintln!("[DEBUG CLIENT] Received response: {}", answer);

                    // Parsear respuesta JSON
                    match ServerMessage::from_json(&answer) {
                        Some(ServerMessage::UserList { users }) => {
                            eprintln!("[DEBUG CLIENT] lista de usuario {}", users);
                            let _ = tx.send(users);
                        }
                        Some(_) => {
                            eprintln!("[DEBUG CLIENT] Unexpected response type");
                            let _ = tx.send("ERROR: Unexpected response".to_string());
                        }
                        None => {
                            eprintln!("[ERROR] Failed to parse response");
                            let _ = tx.send(format!("ERROR: Failed to parse response"));
                        }
                    }
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

        egui::TopBottomPanel::top("menu_bar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.heading("Usuarios Conectados");
                    ui.separator();

                    ui.add_space(ui.available_width() - 100.0);

                    if ui.button("Cerrar Sesión").clicked() {
                        cambio = CambioVista::ToLogin;
                    }
                });
            });

        if cambio != CambioVista::None {
            return cambio;
        }

        cambio = crate::vista::Vista::camera_preview_side_panel(ctx, camara);

        // Tarjeta de usuario
        egui::CentralPanel::default().show(ctx, |ui| {
            Frame::new()
                .fill(Color32::DARK_GRAY)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_height(30.0);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Llamada de Prueba")
                                .strong()
                                .color(Color32::WHITE),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Iniciar llamada").clicked() {
                                cambio = CambioVista::ToCallStart;
                            }
                        });
                    });
                });
        });

        cambio
    }
}
