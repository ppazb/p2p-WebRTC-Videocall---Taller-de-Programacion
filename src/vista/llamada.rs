use std::sync::mpsc::Receiver;

use eframe::egui::{self, Color32, ColorImage, TextureHandle, Vec2};
use opencv::{imgproc, prelude::*};

use crate::{camera::Camera, vista::cambio::CambioVista};

pub struct VistaLlamada {
    // local_cam: Option<Camera>,
    cam_enable: bool,
    // no_cam: Option<TextureHandle>,
    rx_local: Option<Receiver<Mat>>,
    rx_remoto: Option<Receiver<Mat>>,
    tex_local: Option<TextureHandle>,
    tex_remoto: Option<TextureHandle>
}

impl VistaLlamada {
    pub fn new(rx_local: Option<Receiver<Mat>>, rx_remoto: Option<Receiver<Mat>>) -> Self {
        Self {
            // local_cam: None,
            cam_enable: false,
            rx_local,
            rx_remoto,
            tex_local: None,
            tex_remoto: None
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
                match self.cam_enable {
                    true => {
                        if ui.button("Apagar cámara").clicked() {
                            self.liberar_camara();
                            return;
                        }
                    },
                    false => {
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
                // === REMOTE FRAME ===
                // let mut tx_remota = None;

                if let Some(rx) = &self.rx_remoto {
                    if let Ok(mat) = rx.try_recv() {
                        if let Some(color_image) = Self::mat_to_color_image(&mat) {
                            self.tex_remoto = Some(ctx.load_texture(
                                "cam_remota",
                                color_image,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }
                }

                if let Some(tex) = &self.tex_remoto {
                    ui.image((tex.id(), egui::vec2(ui.available_width() * 0.4, ui.available_height() * 0.5)));
                } else {
                    ui.heading("No Camera");
                }

                ui.add_space(20.0);
                
                // Video local
                match self.cam_enable {
                    true => {
                        // === LOCAL FRAME ===
                        //let mut tx_local = None;

                        if let Some(rx) = &self.rx_local {
                            if let Ok(mat) = rx.try_recv() {
                                if let Some(color_image) = Self::mat_to_color_image(&mat) {
                                    // Crear o reemplazar textura
                                    self.tex_local = Some(ctx.load_texture(
                                        "cam_local",
                                        color_image,
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                            }
                        }

                        // Leo estado de cámara, cargo datos en frame
                        // if let Some(rx) = self.rx_local {
                        //     let frame = rx.recv().unwrap();
                        // }
                        // let textura = self.generar_textura(ctx, frame);

                        if let Some(tex) = &self.tex_local {
                            ui.image((tex.id(), egui::vec2(ui.available_width() * 0.4, ui.available_height() * 0.5)));
                        } else {
                            ui.heading("No Camera");
                        }
                    },
                    false => {
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
        self.cam_enable = false;
        // self.local_cam = None;
    }

    fn cargar_camara(&mut self) {
        self.cam_enable = true;
        // self.local_cam = Some(Camera::new().unwrap());
    }

    // fn cargar_frame(&mut self) -> Mat {
    //     let mut frame = opencv::core::Mat::default();
        
    //     if let Some(cam) = &mut self.local_cam {
    //         frame = cam.leer_frame();
    //     }
    //     return frame;
    // }

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

    fn mat_to_color_image(mat: &Mat) -> Option<ColorImage> {
        // Aseguramos formato RGB
        let mut rgb = Mat::default();
        if mat.channels() == 3 {
            imgproc::cvt_color(mat, &mut rgb, imgproc::COLOR_BGR2RGB, 0).ok()?;
        } else {
            return None;
        }

        // Tamaño
        let size = rgb.size().ok()?;
        let width = size.width as usize;
        let height = size.height as usize;

        // Extraer bytes
        let data = rgb.data_bytes().ok()?;
        Some(ColorImage::from_rgb([width, height], data))
    }
    
}