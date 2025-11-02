use eframe::egui::{self, Color32, ColorImage, Frame, Margin, Vec2};

use crate::{camera::Camera, vista::{call_end::VistaCallEnd, call_start::VistaCallStart, cambio::CambioVista, llamada::VistaLlamada, lobby::VistaLobby, login::VistaLogin}};
use opencv::{core::{Mat, MatTraitConst, MatTraitConstManual}, imgproc};

pub mod cambio;
pub mod llamada;
pub mod login;
pub mod lobby;
pub mod call_end;
pub mod call_start;

pub enum Vista {
    VistaLogin(VistaLogin),
    VistaLobby(VistaLobby),
    VistaCallStart(VistaCallStart),
    VistaLlamada(VistaLlamada),
    VistaCallEnd(VistaCallEnd),
}

impl Vista {
    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Camera>) -> CambioVista {
        match self {
            Vista::VistaLogin(view) => view.ui(ctx),
            Vista::VistaLobby(view) => view.ui(ctx, camara),
            Vista::VistaLlamada(view) => view.ui(ctx),
            Vista::VistaCallStart(view) => view.ui(ctx, camara),
            Vista::VistaCallEnd(view) => view.ui(ctx),
        }
    }

    fn generar_imagen(frame: &Mat) -> Option<ColorImage> {
        // Convierto de BGR a RGB
        let mut rgb = Mat::default();
        if frame.channels() == 3 {
            imgproc::cvt_color(frame, &mut rgb, imgproc::COLOR_BGR2RGB, 0).ok()?;
        } else {
            return None;
        }

        // Defino dimensiones
        let size = rgb.size().ok()?;
        let width = size.width as usize;
        let height = size.height as usize;

        // Genero imagen a partir de bytes
        let data = rgb.data_bytes().ok()?;
        Some(ColorImage::from_rgb([width, height], data))
    }

    fn no_camera_texture() -> Option<ColorImage> {
        // Recuadro gris 320x240
        // Se crea una vez al instanciar la vista
        let gray = Color32::from_gray(60);
        let image = ColorImage {
            size: [320, 240],
            pixels: vec![gray; 320 * 240],
            source_size: Vec2 { x: 320.0, y: 240.0 },
        };
        Some(image)
    }

    fn camera_preview_side_panel(ctx: &egui::Context, camera: &Option<Camera>) -> CambioVista{
        let mut cambio = CambioVista::None;
        
        egui::SidePanel::right("preview").resizable(false).show_separator_line(false).default_width(340.0).frame(
            Frame::default()
            .inner_margin(Margin::same(0)) // elimina márgenes internos
            .outer_margin(Margin::same(0)), // elimina espacio externo
        ).show(ctx, |ui| {
            // Dibuja una línea vertical al borde izquierdo del panel
            let rect = ui.max_rect();
            let stroke_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left(), rect.top()),
                    egui::pos2(rect.left(), rect.bottom()),
                ],
                egui::Stroke { width: 1.0, color: stroke_color },
            );
            
            // Vista previa de cámara + controles
            ui.vertical_centered(|ui| {
                ui.set_width(ui.available_width());

                let top_space = (ui.available_height() - 240.0 - 10.0).max(0.0) / 2.0;
                ui.add_space(top_space);

                if let Some(cam) = &camera {
                    

                    if cam.is_active() {
                        let frame = cam.get_last_frame();

                        let imagen;
                        if let Some(f) = &frame {
                            imagen = Self::generar_imagen(f);
                        } else {
                            imagen = Self::no_camera_texture();
                        }

                        if let Some(img) = imagen {
                            let textura = ctx.load_texture("cam_local", img, egui::TextureOptions::LINEAR);
                            ui.image((textura.id(), egui::vec2(320.0, 240.0)));
                        };

                        ui.add_space(10.0);

                        if ui.button("Apagar cámara").clicked() {
                            cambio = CambioVista::CamDisable;
                        }
                    } else {
                        let imagen = Self::no_camera_texture();

                        if let Some(img) = imagen {
                            let textura = ctx.load_texture("cam_local", img, egui::TextureOptions::LINEAR);
                            ui.image((textura.id(), egui::vec2(320.0, 240.0)));
                        };

                        ui.add_space(10.0);

                        if ui.button("Encender cámara").clicked() {
                            cambio = CambioVista::CamEnable;
                        }
                    }

                    // let imagen;
                    // if let Some(f) = &frame {
                    //     imagen = Self::generar_imagen(f);
                    // } else {
                    //     imagen = Self::no_camera_texture();
                    // }

                    // if let Some(img) = imagen {
                    //     let textura = ctx.load_texture("cam_local", img, egui::TextureOptions::LINEAR);
                    //     ui.image((textura.id(), egui::vec2(320.0, 240.0)));
                    // };

                    // ui.add_space(10.0);

                    // if cam.is_active() {
                    //     if ui.button("Apagar cámara").clicked() {
                    //         cambio = CambioVista::CamDisable;
                    //     }
                    // } else {
                    //     if ui.button("Encender cámara").clicked() {
                    //         cambio = CambioVista::CamEnable;
                    //     }
                    // }
                } 
                
                
                
                
                
                
                
                // else {
                //     if let Some(img) = Self::no_camera_texture() {
                //         let textura = ctx.load_texture("cam_local", img, egui::TextureOptions::LINEAR);
                //         ui.image((textura.id(), egui::vec2(320.0, 240.0)));
                //     }

                //     ui.add_space(10.0);

                //     if ui.button("Encender cámara").clicked() {
                //         cambio = CambioVista::CamEnable;
                //     }
                // }

            });
        });

        return cambio;
    }
}
