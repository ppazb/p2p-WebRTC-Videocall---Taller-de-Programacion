use eframe::egui::{self, ColorImage, Frame, Margin};
use crate::{camera::Camera, vista::cambio::{self, CambioVista}};

pub struct VistaLobby {
    no_camera_img: Option<ColorImage>,
}

impl VistaLobby {
    pub fn new() -> Self {
        Self {
            no_camera_img: crate::vista::Vista::no_camera_texture(),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Camera>) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::TopBottomPanel::top("menu_bar").exact_height(50.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("Usuarios Conectados");
                ui.separator();
                
                ui.add_space(ui.available_width()-100.0);

                if ui.button("Cerrar Sesión").clicked() {
                    cambio = CambioVista::ToLogin;
                }
            });
        });

        if cambio != CambioVista::None {
            return cambio;
        }

        cambio = crate::vista::Vista::camera_preview_side_panel(ctx, camara);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Contenido principal de la vista
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() * 0.6);

                    if ui.button("Iniciar llamada").clicked() {
                        cambio = CambioVista::ToCallStart;
                    }
                });
            });
        });

        return cambio;
    }
}