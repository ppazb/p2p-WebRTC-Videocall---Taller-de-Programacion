use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, Frame, RichText};

use crate::{camera::Camera, vista::cambio::CambioVista};

pub struct VistaCalledUserBusy {}

impl Default for VistaCalledUserBusy {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaCalledUserBusy {
    pub fn new() -> Self {
        Self {}
    }
    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Arc<Mutex<Camera>>>) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::TopBottomPanel::top("menu_bar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.heading("Llamada en Curso");
                    ui.separator();

                    ui.add_space(ui.available_width() - 100.0);
                });
            });

        // Mostramos la vista previa de la cámara local mientras esperamos
        crate::vista::Vista::camera_preview_side_panel(ctx, camara);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                Frame::new()
                    .fill(Color32::DARK_GRAY)
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(20))
                    .show(ui, |ui| {
                        ui.set_width(300.0);
                        ui.set_height(150.0);

                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);

                            ui.label(
                                RichText::new("El usuario está ocupado.")
                                    .strong()
                                    .size(18.0)
                                    .color(Color32::WHITE),
                            );

                            ui.add_space(30.0);

                            if ui
                                .button(RichText::new(" Volver al lobby").color(Color32::RED))
                                .clicked()
                            {
                                cambio = CambioVista::ToLobby {
                                    username: String::new(),
                                };
                            }
                        });
                    });
            });
        });

        cambio
    }
}
