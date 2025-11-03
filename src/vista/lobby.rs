use crate::{camera::Camera, vista::cambio::CambioVista};
use eframe::egui::{self, Align, Color32, Frame, Layout, RichText};

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

    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Camera>) -> CambioVista {
        let mut cambio = CambioVista::None;

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
