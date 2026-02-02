use std::{
    net::TcpStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    time::Instant,
};

use eframe::egui::{self, Color32, ColorImage, Frame, Margin, RichText, TextureHandle, Vec2};
use opencv::prelude::*;
use rustls::StreamOwned;

use crate::{
    audio_processing::AudioSystem,
    camera::Camera,
    client::RtcStats,
    vista::{CAPTURE_ASPECT_RATIO, cambio::CambioVista},
};

pub struct VistaLlamada {
    rx_remoto: Option<Receiver<Mat>>,
    last_remote_img: Option<ColorImage>,
    last_local_img: Option<ColorImage>,
    no_camera_img: Option<ColorImage>,
    tex_remoto: Option<TextureHandle>,
    tex_local: Option<TextureHandle>,
    in_call: Arc<AtomicBool>,
    stats: Arc<Mutex<RtcStats>>,
}

impl VistaLlamada {
    pub fn new(
        rx_remoto: Option<Receiver<Mat>>,
        in_call: Arc<AtomicBool>,
        stats: Arc<Mutex<RtcStats>>,
    ) -> Self {
        Self {
            rx_remoto,
            last_remote_img: None,
            last_local_img: None,
            no_camera_img: crate::vista::Vista::no_camera_texture(),
            tex_remoto: None,
            tex_local: None,
            in_call,
            stats,
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        camera: &Option<Arc<Mutex<Camera>>>,
        audio_system: &Arc<AudioSystem>,
        _tls_tcp: &mut Arc<std::sync::Mutex<StreamOwned<rustls::ClientConnection, TcpStream>>>,
    ) -> CambioVista {
        // Reduzco demora de tooltip de estadísticas
        ctx.style_mut(|style| {
            style.interaction.tooltip_delay = 0.05;
        });

        let mut cambio = CambioVista::None;

        egui::TopBottomPanel::bottom("controls")
            .exact_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Consulta de estado
                    let cam_active = if let Some(cam) = camera {
                        if let Ok(cam) = cam.lock() {
                            cam.is_active()
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if cam_active {
                        if ui
                            .add_sized([150.0, 32.0], egui::Button::new("Apagar cámara"))
                            .clicked()
                        {
                            cambio = CambioVista::CamDisable;
                        }
                    } else if ui
                        .add_sized([150.0, 32.0], egui::Button::new("Encender cámara"))
                        .clicked()
                    {
                        cambio = CambioVista::CamEnable;
                    }

                    if !audio_system.is_capturing() {
                        if ui
                            .add_sized([150.0, 32.0], egui::Button::new("Encender micrófono"))
                            .clicked()
                        {
                            cambio = CambioVista::MicEnable;
                        }
                    } else if ui
                        .add_sized([150.0, 32.0], egui::Button::new("Apagar micrófono"))
                        .clicked()
                    {
                        cambio = CambioVista::MicDisable;
                    }

                    if ui
                        .add_sized([150.0, 32.0], egui::Button::new("Enviar archivo"))
                        .clicked()
                    {
                        cambio = CambioVista::SendFile;
                    }

                    if ui
                        .add_sized(
                            [150.0, 32.0],
                            egui::Button::new(
                                RichText::new("Finalizar llamada").color(Color32::WHITE),
                            )
                            .fill(Color32::DARK_RED),
                        )
                        .clicked()
                    {
                        cambio = CambioVista::ToCallEnd;

                        // Cambio el estado del flag, notificando a los threads del cliente
                        self.in_call.store(false, Ordering::SeqCst);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                    });
                })
            });

        // Tamaño proporcional del panel lateral
        let screen_width = ctx.content_rect().width();
        let panel_ratio = 0.35;
        let panel_width = screen_width * panel_ratio;

        egui::SidePanel::right("preview")
            .resizable(true)
            .show_separator_line(true)
            .min_width(panel_width)
            .default_width(panel_width)
            .max_width(screen_width * (1.0 - panel_ratio))
            .frame(
                Frame::default()
                    .inner_margin(Margin::same(0))
                    .outer_margin(Margin::same(0)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Carga de frame
                    let latest_frame = if let Some(cam) = camera {
                        if let Ok(cam) = cam.lock() {
                            cam.get_last_frame()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Actualizo imagen local a partir del frame obtenido
                    if let Some(frame) = latest_frame.as_ref() {
                        self.last_local_img = crate::vista::Vista::generar_imagen(frame);
                    } else {
                        self.last_local_img = None;
                    }

                    let image = self.last_local_img.as_ref().or(self.no_camera_img.as_ref());

                    // Muestro la imagen contenida en el panel
                    Self::in_call_camera_feed(
                        ctx,
                        ui,
                        &mut self.tex_local,
                        "cam_local".to_string(),
                        image,
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Carga de frame
                let latest_img = if let Some(rx) = &self.rx_remoto
                    && let Ok(frame) = rx.try_recv()
                {
                    let img = crate::vista::Vista::generar_imagen(&frame);
                    self.last_remote_img = img;
                    self.last_remote_img.as_ref()
                } else {
                    let now = Instant::now();
                    if let Ok(s) = self.stats.lock()
                        && let Some(last_arrival) = s.last_arrival_time
                        && now.duration_since(last_arrival).as_millis() as f64 > 3000.0
                    {
                        self.no_camera_img.as_ref()
                    } else {
                        self.last_remote_img.as_ref()
                    }
                };

                // Cargo imagen a partir del frame obtenido
                Self::in_call_camera_feed(
                    ctx,
                    ui,
                    &mut self.tex_remoto,
                    "cam_remota".to_string(),
                    latest_img,
                );
            });
        });
        cambio
    }

    fn in_call_camera_feed(
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tex: &mut Option<TextureHandle>,
        id: String,
        image: Option<&ColorImage>,
    ) {
        // Mediciones de vista previa
        let aspect_ratio = CAPTURE_ASPECT_RATIO;

        // Calculo dimensiones en base al espacio disponible
        let max_size = ui.available_size();
        let side_padding = 40.0;

        let max_width = (max_size.x - side_padding).max(0.0);
        let max_height = (max_size.y).max(0.0);

        // Limito valores por aspect ratio
        let preview_size = if (max_width / max_height) > aspect_ratio {
            // Mas ancho que alto, recalculo en base al alto
            egui::vec2(max_height * aspect_ratio, max_height)
        } else {
            // Mas alto que ancho, recalculo en base al ancho
            egui::vec2(max_width, max_width / aspect_ratio)
        };

        let top_space = ((ui.available_height() - preview_size.y).max(20.0)) / 2.0;
        ui.add_space(top_space);

        // Actualizo textura recibida de la vista
        if let Some(img) = image {
            let texture = tex.get_or_insert_with(|| {
                ctx.load_texture(id, img.clone(), egui::TextureOptions::LINEAR)
            });
            texture.set(img.clone(), egui::TextureOptions::LINEAR);

            // Dibujo la textura en un recuadro del tamaño calculado
            let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
            crate::vista::Vista::draw_frame_aspect_fill(
                ui,
                texture,
                egui::vec2(img.width() as f32, img.height() as f32), // tamaño real del frame captado
                rect,
            );
        }
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
                }
                j if j < 40.0 => {
                    color = Color32::YELLOW;
                }
                _ => {
                    color = Color32::RED;
                }
            }

            // Construyo el icono de jitter
            let widget = Self::jitter_indicator(ui, color);

            // Completo tooltip con más estadísticas
            widget
                .on_hover_ui(|ui| {
                    ui.set_min_width(150.0);

                    ui.label("Recepción (Video)");
                    ui.separator();

                    ui.label(format!("Jitter: {:.2} ms", s.get_jitter()));
                    ui.label(format!(
                        "Bitrate: {}",
                        Self::format_bitrate(s.get_bitrate())
                    ));
                    ui.label(format!("Paquetes: {}", Self::format_pps(s.get_pps())));
                    ui.separator();

                    // ui.label(format!(
                    //     "Totales:\n\
                    //     \t- Paquetes: {}\n\
                    //     \t- Bytes: {}",
                    //     s.packets_received,
                    //     s.bytes_received,
                    // ));
                })
                .hover_pos();
        }
    }

    fn jitter_indicator(ui: &mut egui::Ui, color: Color32) -> egui::Response {
        // Defino dimensiones y margenes del icono
        let diameter = 15.0;
        let margin_left = 0.0;
        let margin_top = 0.0;

        // Defino dimensiones del elemento interactivo
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(diameter + margin_left, diameter + margin_top),
            egui::Sense::hover(),
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
