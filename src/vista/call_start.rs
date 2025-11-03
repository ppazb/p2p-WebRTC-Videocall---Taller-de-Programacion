use eframe::egui::{self, Align, Color32, Frame, Layout, RichText};

use crate::{camera::Camera, vista::cambio::CambioVista};

pub struct VistaCallStart {}

impl Default for VistaCallStart {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaCallStart {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Camera>) -> CambioVista {
        let mut cambio;

        egui::TopBottomPanel::top("menu_bar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.heading("Llamada en Curso");
                    ui.separator();

                    ui.add_space(ui.available_width() - 100.0);
                });
            });

        cambio = crate::vista::Vista::camera_preview_side_panel(ctx, camara);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
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
                                if ui.button("Cancelar llamada").clicked() {
                                    cambio = CambioVista::ToLobby;
                                }
                            });
                        });
                    });

                ui.heading("Esperando conexión...");
            });
        });

        cambio
    }
}
