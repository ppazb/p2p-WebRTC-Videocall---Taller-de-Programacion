use eframe::egui::{self, Color32, ColorImage, TextureHandle, Vec2};
use opencv::prelude::*;

use crate::{camera::Camera, vista::cambio::CambioVista};

pub struct VistaLlamada {
    local_cam: Option<Camera>,
    cam_enable: bool,
    // no_cam: Option<TextureHandle>
}

impl VistaLlamada {
    pub fn new() -> Self {
        Self {
            local_cam: None,
            cam_enable: false,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista{
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            // Controles llamada
            ui.horizontal(|ui| {
                // Botón Salida
                if ui.button("Salir").clicked() {
                    self.liberar_camara();
                    cambio = CambioVista::Login;
                    return;
                }

                // Toggle cámara
                match self.local_cam {
                    Some(_) => {
                        if ui.button("Apagar cámara").clicked() {
                            self.liberar_camara();
                            return;
                        }
                    },
                    None => {
                        if ui.button("Encender cámara").clicked() {
                            self.cargar_camara();
                        }
                    }
                }
            });
            
            ui.separator();

            // Frame -> Imagen -> Textura -> Widget
            // Capturas video
            ui.horizontal_centered(|ui| {
                // Video remoto

                ui.add_space(20.0);
                
                // Video local
                match self.local_cam {
                    Some(_) => {
                        // Leo estado de cámara, cargo datos en frame
                        let frame = self.cargar_frame();
                        let textura = self.generar_textura(ctx, frame);

                        if let Some(tex) = &textura {
                            ui.image((tex.id(), egui::vec2(ui.available_width() * 0.4, ui.available_height() * 0.5)));
                        }
                    },
                    None => {
                        ui.heading("No Camera");

                        // Cargar la textura no_camera y posicionar acá
                    }
                }
            });
            

            ctx.request_repaint();
        });

        return cambio;
    }

    fn liberar_camara(&mut self) {
        self.local_cam = None;
    }

    fn cargar_camara(&mut self) {
        self.local_cam = Some(Camera::new().unwrap());
    }

    fn cargar_frame(&mut self) -> Mat {
        let mut frame = opencv::core::Mat::default();
        
        if let Some(cam) = &mut self.local_cam {
            frame = cam.leer_frame();
        }
        return frame;
    }

    fn generar_textura(&mut self, ctx: &egui::Context, frame: Mat) -> Option<TextureHandle> {
        // Convierto Mat a imagen
        let size = frame.size().unwrap();
        let data = frame.data_bytes().unwrap();

        let image = egui::ColorImage::from_rgb(
            [size.width as usize, size.height as usize],
            data,
        );

        // Genero textura en egui
        return Some(ctx.load_texture("webcam", image, egui::TextureOptions::LINEAR_MIRRORED_REPEAT));
    }

    fn no_camera_texture(ctx: &egui::Context) -> Option<TextureHandle> {
        // Recuadro gris 320x240
        // Genero única vez durante la creación de la vista
        let gray = Color32::from_gray(60);
        let image = ColorImage {
            size: [320, 240],
            pixels: vec![gray; 320 * 240],
            source_size: Vec2 { x: 320.0, y: 240.0 },
        };
        Some(ctx.load_texture("no_camera", image, egui::TextureOptions::LINEAR))
    }
    
}