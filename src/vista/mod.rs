use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
};

use eframe::egui::{self, Color32, ColorImage, Frame, Margin, TextureHandle, Vec2};
use rustls::StreamOwned;

use crate::{
    audio_processing::AudioSystem,
    camera::Camera,
    vista::{
        call_end::VistaCallEnd, call_start::VistaCallStart, called_user_busy::VistaCalledUserBusy,
        cambio::CambioVista, file_sending::VistaSendFile, incoming_call::VistaIncomingCall,
        llamada::VistaLlamada, lobby::VistaLobby, login::VistaLogin, sign_up::VistaSignUp,
    },
};
use opencv::{
    core::{Mat, MatTraitConst, MatTraitConstManual},
    imgproc,
};

// Relación de aspecto para capturas de video
const CAPTURE_ASPECT_RATIO: f32 = 4.0 / 3.0;

pub mod call_end;
pub mod call_start;
pub mod called_user_busy;
pub mod cambio;
pub mod file_sending;
pub mod incoming_call;
pub mod llamada;
pub mod lobby;
pub mod login;
pub mod sign_up;

pub enum Vista {
    VistaLogin(VistaLogin),
    VistaSignUp(VistaSignUp),
    VistaLobby(VistaLobby),
    VistaCallStart(VistaCallStart),
    VistaLlamada(VistaLlamada),
    VistaCallEnd(VistaCallEnd),
    VistaIncomingCall(VistaIncomingCall),
    VistaCalledUserBusy(VistaCalledUserBusy),
    VistaSendFile(VistaSendFile),
    VistaFileSendingRequested(String, String),
    VistaTargetUserFileSendingAccepted(String, String),
    VistaTargetUserFileSendingRejected(String, String),
}

impl Vista {
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        camara: &Option<Arc<Mutex<Camera>>>,
        tls_tcp: &mut Arc<std::sync::Mutex<StreamOwned<rustls::ClientConnection, TcpStream>>>,
        audio_system: &Arc<AudioSystem>,
    ) -> CambioVista {
        match self {
            Vista::VistaLogin(view) => view.ui(ctx, tls_tcp),
            Vista::VistaSignUp(view) => view.ui(ctx, tls_tcp),
            Vista::VistaLobby(view) => view.ui(ctx, camara, audio_system, tls_tcp),
            Vista::VistaLlamada(view) => view.ui(ctx, camara, audio_system, tls_tcp),
            Vista::VistaCallStart(view) => view.ui(ctx, camara, audio_system),
            Vista::VistaCallEnd(view) => view.ui(ctx, camara, audio_system, tls_tcp),
            Vista::VistaIncomingCall(view) => view.ui(ctx, camara, tls_tcp),
            Vista::VistaCalledUserBusy(view) => view.ui(ctx, camara, audio_system),
            Vista::VistaSendFile(view) => view.ui(ctx),
            Vista::VistaFileSendingRequested(from_user, filename) => {
                Self::file_sending_requested_pop_up(ctx, from_user, filename)
            }
            Vista::VistaTargetUserFileSendingAccepted(from_user, filename) => {
                Self::file_sending_accepted_pop_up(ctx, from_user, filename)
            }
            Vista::VistaTargetUserFileSendingRejected(from_user, filename) => {
                Self::file_sending_rejected_pop_up(ctx, from_user, filename)
            }
        }
    }

    fn file_sending_requested_pop_up(
        ctx: &egui::Context,
        from_user: &str,
        filename: &str,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::Window::new("Solicitud de envío de archivo")
            .collapsible(true)
            .resizable(true)
            .default_width(300.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(format!(
                    "Usuario {} quiere enviarte un archivo: {}",
                    from_user, filename
                ));

                ui.horizontal(|ui| {
                    if ui.button("Aceptar").clicked() {
                        // Lógica para aceptar el envío
                        cambio = CambioVista::SendFileAccepted {
                            from_user: from_user.to_string(),
                            filename: filename.to_string(),
                        };
                    }
                    if ui.button("Rechazar").clicked() {
                        // Lógica para rechazar el envío
                        cambio = CambioVista::SendFileRejected {
                            from_user: from_user.to_string(),
                            filename: filename.to_string(),
                        };
                    }
                });
            });

        cambio
    }

    fn file_sending_accepted_pop_up(
        ctx: &egui::Context,
        from_user: &str,
        filename: &str,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::Window::new("Solicitud Aceptada")
            .collapsible(true)
            .resizable(true)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(format!(
                    "El usuario {} ha aceptado recibir el archivo: {}",
                    from_user, filename
                ));

                if ui.button("Cerrar").clicked() {
                    cambio = CambioVista::SendFileNoticeClosed;
                }
            });

        cambio
    }

    fn file_sending_rejected_pop_up(
        ctx: &egui::Context,
        from_user: &str,
        filename: &str,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::Window::new("Solicitud Rechazada")
            .collapsible(true)
            .resizable(true)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(format!(
                    "El usuario {} ha rechazado recibir el archivo: {}",
                    from_user, filename
                ));

                if ui.button("Cerrar").clicked() {
                    cambio = CambioVista::None;
                }
            });

        cambio
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

    pub fn camera_preview_side_panel(
        ctx: &egui::Context,
        camera: &Option<Arc<Mutex<Camera>>>,
        audio_system: &Arc<AudioSystem>,
        texture: &mut Option<TextureHandle>,
    ) -> CambioVista {
        let mut cambio = CambioVista::None;

        // Tamaño proporcional del panel lateral
        let screen_width = ctx.content_rect().width();
        let panel_ratio = 0.35;
        let panel_width = screen_width * panel_ratio;

        egui::SidePanel::right("preview")
            .resizable(false)
            .show_separator_line(false)
            .exact_width(panel_width)
            .frame(
                Frame::default()
                    .inner_margin(Margin::same(0))
                    .outer_margin(Margin::same(0)),
            )
            .show(ctx, |ui| {
                // Línea separadora al borde izquierdo del panel
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
                    // Mediciones de vista previa
                    let aspect_ratio = CAPTURE_ASPECT_RATIO;

                    // Calculo dimensiones en base al espacio disponible
                    let max_size = ui.available_size();
                    let buttons_height = 100.0;
                    let side_padding = 40.0;

                    let max_width = (max_size.x - side_padding).max(0.0);
                    let max_height = (max_size.y - buttons_height).max(0.0);

                    // Limito valores por aspect ratio
                    let preview_size = if (max_width / max_height) > aspect_ratio {
                        // Mas ancho que alto, recalculo en base al alto
                        egui::vec2(max_height * aspect_ratio, max_height)
                    } else {
                        // Mas alto que ancho, recalculo en base al ancho
                        egui::vec2(max_width, max_width / aspect_ratio)
                    };

                    // Carga de frame y consulta de estado
                    let (latest_frame, cam_active) = if let Some(cam) = camera {
                        if let Ok(cam) = cam.lock() {
                            (cam.get_last_frame(), cam.is_active())
                        } else {
                            (None, false)
                        }
                    } else {
                        (None, false)
                    };

                    // Cargo imagen a partir del frame obtenido
                    let image = latest_frame
                        .as_ref()
                        .and_then(Self::generar_imagen)
                        .or_else(Self::no_camera_texture);

                    let content_height = buttons_height + preview_size.y + 10.0;
                    let top_space = ((ui.available_height() - content_height).max(20.0)) / 2.0;
                    ui.add_space(top_space);

                    // Actualizo textura recibida de la vista
                    if let Some(img) = image {
                        let texture = texture.get_or_insert_with(|| {
                            ctx.load_texture("cam_local", img.clone(), egui::TextureOptions::LINEAR)
                        });
                        texture.set(img.clone(), egui::TextureOptions::LINEAR);

                        // Dibujo la textura en un recuadro del tamaño calculado
                        let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
                        Self::draw_frame_aspect_fill(
                            ui,
                            texture,
                            egui::vec2(img.width() as f32, img.height() as f32), // tamaño real del frame captado
                            rect,
                        );
                    }

                    ui.add_space(10.0);

                    // Controles
                    if cam_active {
                        if ui
                            .add_sized([200.0, 32.0], egui::Button::new("Apagar cámara"))
                            .clicked()
                        {
                            cambio = CambioVista::CamDisable;
                        }
                    } else if ui
                        .add_sized([200.0, 32.0], egui::Button::new("Encender cámara"))
                        .clicked()
                    {
                        cambio = CambioVista::CamEnable;
                    }

                    if !audio_system.is_capturing() {
                        if ui
                            .add_sized([200.0, 32.0], egui::Button::new("Encender micrófono"))
                            .clicked()
                        {
                            cambio = CambioVista::MicEnable;
                        }
                    } else {
                        if ui
                            .add_sized([200.0, 32.0], egui::Button::new("Apagar micrófono"))
                            .clicked()
                        {
                            cambio = CambioVista::MicDisable;
                        }
                        if ui
                            .checkbox(&mut audio_system.is_playing_back(), "Escuchar audio")
                            .changed()
                        {
                            cambio = CambioVista::PlaybackToggle {
                                state: !audio_system.is_playing_back(),
                            };
                        }
                    }
                });
            });

        cambio
    }

    fn draw_frame_aspect_fill(
        ui: &mut egui::Ui,
        texture: &egui::TextureHandle,
        frame_size: egui::Vec2,
        rect: egui::Rect,
    ) {
        let rect_size = rect.size();

        // La escala indica qué proporción del frame puede dibujarse en el recuadro
        let scale = (rect_size.x / frame_size.x).max(rect_size.y / frame_size.y);
        let scaled_size = frame_size * scale;

        // Sobrante por ajuste de aspect ratio
        let excess = scaled_size - rect_size;

        // Mapeo UVs (0..1) para hacer crop centrado del frame
        let uv_min = egui::pos2(
            (excess.x / scaled_size.x) / 2.0,
            (excess.y / scaled_size.y) / 2.0,
        );
        let uv_max = egui::pos2(1.0 - uv_min.x, 1.0 - uv_min.y);

        // Dibujo la textura
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(uv_min, uv_max),
            egui::Color32::WHITE,
        );
    }
}
