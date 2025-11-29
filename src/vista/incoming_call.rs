use crate::{camera::Camera, vista::cambio::CambioVista};
use eframe::egui::{self, Color32, Frame, RichText, Vec2};
use rustls::StreamOwned;
use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
};

pub struct VistaIncomingCall {
    caller_name: String,
}

//Igual a loby pero con el cartel de aceptar
impl VistaIncomingCall {
    pub fn new(caller_name: String) -> Self {
        Self { caller_name }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        _camara: &Option<Arc<Mutex<Camera>>>,
        _tls_tcp: &mut Arc<std::sync::Mutex<StreamOwned<rustls::ClientConnection, TcpStream>>>,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            let frame = Frame::new()
                .fill(Color32::from_black_alpha(240))
                .inner_margin(egui::Margin::same(20))
                .corner_radius(10.0);

            frame.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);

                    ui.label(
                        RichText::new("📞 Llamada Entrante")
                            .color(Color32::LIGHT_BLUE)
                            .size(24.0)
                            .strong(),
                    );
                    ui.add_space(20.0);

                    ui.label(
                        RichText::new(&self.caller_name)
                            .color(Color32::WHITE)
                            .size(32.0)
                            .strong(),
                    );

                    ui.add_space(50.0);

                    ui.horizontal(|ui| {
                        let width = ui.available_width() / 2.0 - 10.0;

                        if ui
                            .add_sized(
                                Vec2::new(width, 50.0),
                                egui::Button::new(
                                    RichText::new("Rechazar").color(Color32::WHITE).size(18.0),
                                )
                                .fill(Color32::DARK_RED),
                            )
                            .clicked()
                        {
                            cambio = CambioVista::CallRejected {
                                target_user: self.caller_name.clone(),
                            };
                        }

                        ui.add_space(20.0);

                        if ui
                            .add_sized(
                                Vec2::new(width, 50.0),
                                egui::Button::new(
                                    RichText::new("Aceptar").color(Color32::BLACK).size(18.0),
                                )
                                .fill(Color32::GREEN),
                            )
                            .clicked()
                        {
                            cambio = CambioVista::CallAccepted {
                                target_user: self.caller_name.clone(),
                            };
                        }
                    });
                });
            });
        });

        cambio
    }
}
