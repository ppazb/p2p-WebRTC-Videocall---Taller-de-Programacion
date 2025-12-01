use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::Receiver,
};

use eframe::egui::{self, Color32, Frame, Margin, TextureHandle, Vec2};
use opencv::prelude::*;

use crate::{camera::Camera, client::RtcStats, vista::cambio::CambioVista};

pub struct VistaLlamada {
    rx_remoto: Option<Receiver<Mat>>,
    tex_remoto: Option<TextureHandle>,
    in_call: Arc<AtomicBool>,
    stats: Arc<Mutex<RtcStats>>,
}

impl VistaLlamada {
    pub fn new(rx_remoto: Option<Receiver<Mat>>, in_call: Arc<AtomicBool>, stats: Arc<Mutex<RtcStats>>) -> Self {
        Self {
            rx_remoto,
            tex_remoto: None,
            in_call,
            stats
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        camara: &Option<Arc<Mutex<Camera>>>,
        _tcp: &mut std::net::TcpStream,
    ) -> CambioVista {
        // Reduzco demora de tooltip de estadísticas
        ctx.style_mut(|style| {
            style.interaction.tooltip_delay = 0.05;
        });

        let mut cambio = CambioVista::None;

        egui::SidePanel::right("preview")
            .resizable(false)
            .show_separator_line(false)
            .default_width(340.0)
            .frame(
                Frame::default()
                    .inner_margin(Margin::same(0))
                    .outer_margin(Margin::same(0)),
            )
            .show(ctx, |ui| {
                // Icono de estadísticas
                self.stats_indicator(ui);
                
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

                ui.vertical_centered(|ui| {
                    ui.set_width(ui.available_width());

                    let top_space = (ui.available_height() - 240.0 - 10.0).max(0.0) / 2.0;
                    ui.add_space(top_space);

                    if let Some(cam) = camara {
                        if let Ok(cam) = cam.lock()
                            && cam.is_active()
                        {
                            let frame = cam.get_last_frame();

                            let imagen;
                            if let Some(f) = &frame {
                                imagen = crate::vista::Vista::generar_imagen(f);
                            } else {
                                imagen = crate::vista::Vista::no_camera_texture();
                            }

                            if let Some(img) = imagen {
                                let textura = ctx.load_texture(
                                    "cam_local",
                                    img,
                                    egui::TextureOptions::LINEAR,
                                );
                                ui.image((textura.id(), egui::vec2(320.0, 240.0)));
                            };

                            ui.add_space(10.0);

                            if ui.button("Apagar cámara").clicked() {
                                cambio = CambioVista::CamDisable;
                            }
                        } else {
                            let imagen = crate::vista::Vista::no_camera_texture();

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

                        ui.add_space(10.0);

                        if ui.button("Finalizar Llamada").clicked() {
                            cambio = CambioVista::ToCallEnd;

                            // Cambio el estado del flag, notificando a los threads del cliente
                            self.in_call.store(false, Ordering::SeqCst);
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.actualizar_frame_remoto(ctx);

            ui.centered_and_justified(|ui| {
                // Muestro la textura actualizada
                if let Some(tex) = &self.tex_remoto {
                    //ui.image((tex.id(), egui::vec2(ui.available_width() * 0.4, ui.available_height() * 0.5)));
                    ui.image((tex.id(), Self::redimensionar_textura(tex, ui)));
                } else {
                    // Placeholder para cámara apagada
                    if let Some(img) = crate::vista::Vista::no_camera_texture() {
                        let tex = ctx.load_texture("cam_remota", img, egui::TextureOptions::LINEAR);
                        ui.image((tex.id(), Self::redimensionar_textura(&tex, ui)));
                    }
                }
            });
        });

        cambio
    }

    // fn actualizar_frame_local(&mut self, ctx: &egui::Context) {
    //     if let Some(rx) = &self.rx_local
    //         && let Ok(mat) = rx.try_recv()
    //         && let Some(color_image) = crate::vista::Vista::generar_imagen(&mat)
    //     {
    //         // Crear o reemplazar textura
    //         if let Some(tex) = &mut self.tex_local {
    //             tex.set(color_image, egui::TextureOptions::LINEAR);
    //         } else {
    //             self.tex_local =
    //                 Some(ctx.load_texture("cam_local", color_image, egui::TextureOptions::LINEAR));
    //         }
    //     }
    // }

    fn actualizar_frame_remoto(&mut self, ctx: &egui::Context) {
        let mut ultimo_frame = None;

        // Intento leer frame del channel de recepción
        if let Some(rx) = &self.rx_remoto
            && let Ok(frame) = rx.try_recv()
        {
            ultimo_frame = Some(frame);
        }

        // Si se cargó un frame en este update, actualizo la textura
        if let Some(mat) = ultimo_frame {
            if let Some(color_image) = crate::vista::Vista::generar_imagen(&mat) {
                // Crear o reemplazar textura
                if let Some(tex) = &mut self.tex_remoto {
                    tex.set(color_image, egui::TextureOptions::LINEAR);
                } else {
                    self.tex_remoto = Some(ctx.load_texture(
                        "cam_remota",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
        } else {
            // Si no, salteo el frame
        }
    }

    fn redimensionar_textura(tex: &TextureHandle, ui: &egui::Ui) -> Vec2 {
        // Ajusto el tamaño manteniendo la relación 4:3
        let tex_size = tex.size_vec2();
        let max_size = egui::vec2(ui.available_width(), ui.available_height());

        let scale = (max_size.x / tex_size.x).min(max_size.y / tex_size.y);

        tex_size * scale
    }

    fn format_bitrate(bps: f64) -> String {
        if bps < 1000.0 {
            format!("{:.0} bps", bps)
        } else if bps < 1000000.0 {
            format!("{:.1} kbps", bps / 1000.0)
        } else {
            format!("{:.2} Mbps", bps / 1000000.0)
        }
    }

    fn format_pps(pps: f64) -> String {
        format!("{:.0} pkt/s", pps)
    }

    fn stats_indicator(&self, ui: &mut egui::Ui) {
        if let Ok(s) = self.stats.lock() {
            // Obtengo el color del icono
            let color;
            match s.get_jitter() {
                j if j < 15.0 => {
                    color = Color32::DARK_GREEN;
                },
                j if j < 40.0 => {
                    color = Color32::YELLOW;
                },
                _ => {
                    color = Color32::RED;
                },
            }

            // Construyo el icono de jitter
            let widget = Self::jitter_indicator(ui, color);

            // Completo tooltip con más estadísticas
            widget.on_hover_ui(|ui| {
                ui.set_min_width(200.0);

                ui.label("Recepción");
                ui.separator();

                ui.label(format!("Jitter: {:.2} ms", s.get_jitter()));
                ui.label(format!("Bitrate: {}", Self::format_bitrate(s.get_bitrate())));
                ui.label(format!("Paquetes: {}", Self::format_pps(s.get_pps())));
                ui.separator();

                // ui.label(format!(
                //     "Totales:\n\
                //     \t- Paquetes: {}\n\
                //     \t- Bytes: {}",
                //     s.packets_received,
                //     s.bytes_received,
                // ));
            });
        }
    }

    fn jitter_indicator(ui: &mut egui::Ui, color: Color32) -> egui::Response {
        // Defino dimensiones y margenes del icono
        let diameter = 10.0;
        let margin_left = 10.0;
        let margin_top = 10.0;

        // Defino dimensiones del elemento interactivo
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(diameter + margin_left, diameter + margin_top),
            egui::Sense::hover()
        );

        // Calculo posición del ícono circular, y lo dibujo
        let center = egui::pos2(
            rect.left() + margin_left + diameter / 2.0,
            rect.top() + margin_top + diameter / 2.0,
        );

        ui.painter().circle_filled(center, diameter / 2.0, color);

        if response.hovered() {
            ui.painter().circle_stroke(
                center,
                diameter / 2.0,
                egui::Stroke::new(2.0, egui::Color32::WHITE),
            );
        }

        // Devuelvo el indicador de color configurado
        response
    }

}
