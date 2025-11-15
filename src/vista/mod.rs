use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, ColorImage, Frame, Margin, TextureHandle, Ui, Vec2};

use crate::{
    camera::Camera,
    vista::{
        call_end::VistaCallEnd, call_start::VistaCallStart, cambio::CambioVista,
        llamada::VistaLlamada, lobby::VistaLobby, login::VistaLogin,
    },
};
use opencv::{
    core::{Mat, MatTraitConst, MatTraitConstManual},
    imgproc,
};

pub mod call_end;
pub mod call_start;
pub mod cambio;
pub mod llamada;
pub mod lobby;
pub mod login;

pub enum Vista {
    VistaLogin(VistaLogin),
    VistaLobby(VistaLobby),
    VistaCallStart(VistaCallStart),
    VistaLlamada(VistaLlamada),
    VistaCallEnd(VistaCallEnd),
}

impl Vista {
    pub fn ui(&mut self, ctx: &egui::Context, camara: &Option<Arc<Mutex<Camera>>>, tcp: &mut std::net::TcpStream) -> CambioVista {
        match self {
            Vista::VistaLogin(view) => view.ui(ctx, tcp),
            Vista::VistaLobby(view) => view.ui(ctx, camara,tcp),
            Vista::VistaLlamada(view) => view.ui(ctx, camara),
            Vista::VistaCallStart(view) => view.ui(ctx, camara),
            Vista::VistaCallEnd(view) => view.ui(ctx, camara),
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

    fn camera_preview_side_panel(
        ctx: &egui::Context,
        camera: &Option<Arc<Mutex<Camera>>>,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::SidePanel::right("preview")
            .resizable(false)
            .show_separator_line(false)
            .default_width(340.0)
            .frame(
                Frame::default()
                    .inner_margin(Margin::same(0)) // elimina márgenes internos
                    .outer_margin(Margin::same(0)), // elimina espacio externo
            )
            .show(ctx, |ui| {
                // Dibuja una línea vertical al borde izquierdo del panel
                let rect = ui.max_rect();
                let stroke_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.left(), rect.top()),
                        egui::pos2(rect.left(), rect.bottom()),
                    ],
                    egui::Stroke {
                        width: 1.0,
                        color: stroke_color,
                    },
                );

                // Vista previa de cámara + controles
                ui.vertical_centered(|ui| {
                    ui.set_width(ui.available_width());

                    let top_space = (ui.available_height() - 240.0 - 10.0).max(0.0) / 2.0;
                    ui.add_space(top_space);

                    if let Some(cam) = camera {
                        if let Ok(cam) = cam.lock()
                            && cam.is_active()
                        {
                            let frame = cam.get_last_frame();

                            let imagen;
                            if let Some(f) = &frame {
                                imagen = Self::generar_imagen(f);
                            } else {
                                imagen = Self::no_camera_texture();
                            }

                            if let Some(img) = imagen {
                                //img = Self::apply_rounded_mask(&img, 40.0);
                                let textura = ctx.load_texture(
                                    "cam_local",
                                    img,
                                    egui::TextureOptions::LINEAR,
                                );
                                //Self::show_rounded_image(ui, textura, egui::vec2(320.0, 240.0));
                                ui.image((textura.id(), egui::vec2(320.0, 240.0)));
                            };

                            ui.add_space(10.0);

                            if ui.button("Apagar cámara").clicked() {
                                cambio = CambioVista::CamDisable;
                            }
                        } else {
                            let imagen = Self::no_camera_texture();

                            if let Some(img) = imagen {
                                let textura = ctx.load_texture(
                                    "cam_local",
                                    img,
                                    egui::TextureOptions::LINEAR,
                                );
                                ui.image((textura.id(), egui::vec2(320.0, 240.0)));
                            };

                            ui.add_space(10.0);

                            if ui.button("Encender cámara").clicked() {
                                cambio = CambioVista::CamEnable;
                            }
                        }
                    }
                });
            });

        cambio
    }

    fn _apply_rounded_mask(img: &ColorImage, radius: f32) -> ColorImage {
        let (w, h) = (img.size[0] as f32, img.size[1] as f32);
        let mut out = img.clone();

        for y in 0..img.size[1] {
            for x in 0..img.size[0] {
                let xf = x as f32;
                let yf = y as f32;

                // Distancias a los bordes (esquinas)
                let dx = if xf < radius {
                    radius - xf
                } else if xf > w - radius {
                    xf - (w - radius)
                } else {
                    0.0
                };

                let dy = if yf < radius {
                    radius - yf
                } else if yf > h - radius {
                    yf - (h - radius)
                } else {
                    0.0
                };

                // Si estamos dentro del borde redondeado, mantenemos el pixel
                // Si estamos fuera (más allá del radio), lo hacemos transparente
                if dx > 0.0 && dy > 0.0 && (dx * dx + dy * dy).sqrt() > radius {
                    let idx = y * img.size[0] + x;
                    let mut color = out.pixels[idx];
                    color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
                    out.pixels[idx] = color;
                }
            }
        }

        out
    }

    fn _show_rounded_image(ui: &mut Ui, tex: TextureHandle, size: Vec2) {
        let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
        let rounding = egui::CornerRadius::same(20);
        let painter = ui.painter();

        // Fondo y borde
        painter.rect_filled(rect, rounding, egui::Color32::from_gray(30));
        painter.rect_stroke(
            rect,
            rounding,
            egui::Stroke::new(2.0, egui::Color32::LIGHT_GRAY),
            egui::StrokeKind::Outside,
        );

        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}
