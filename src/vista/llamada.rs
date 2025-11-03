use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Receiver,
};

use eframe::egui::{self, Color32, ColorImage, TextureHandle, Vec2};
use opencv::{imgproc, prelude::*};

use crate::vista::cambio::CambioVista;

pub struct VistaLlamada {
    cam_enable: bool,
    rx_local: Option<Receiver<Mat>>,
    rx_remoto: Option<Receiver<Mat>>,
    tex_local: Option<TextureHandle>,
    tex_remoto: Option<TextureHandle>,
    in_call: Arc<AtomicBool>,
}

impl VistaLlamada {
    pub fn new(
        rx_local: Option<Receiver<Mat>>,
        rx_remoto: Option<Receiver<Mat>>,
        in_call: Arc<AtomicBool>,
    ) -> Self {
        Self {
            cam_enable: true,
            rx_local,
            rx_remoto,
            tex_local: None,
            tex_remoto: None,
            in_call,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            // Controles llamada
            ui.horizontal(|ui| {
                // Botón Salida
                if self.in_call.load(Ordering::SeqCst) && ui.button("Finalizar llamada").clicked() {
                    self.liberar_camara();
                    cambio = CambioVista::ToCallEnd;

                    // Cambio el estado del flag, notificando a los threads del cliente
                    self.in_call.store(false, Ordering::SeqCst);
                    return;
                }

                // Toggle cámara
                match self.cam_enable {
                    true => {
                        if ui.button("Apagar cámara").clicked() {
                            self.liberar_camara();
                        }
                    }
                    false => {
                        if ui.button("Encender cámara").clicked() {
                            self.cargar_camara();
                        }
                    }
                }
            });

            ui.separator();

            // Widgets video
            ui.horizontal_centered(|ui| {
                // Actualizo frame remoto
                if let Some(rx) = &self.rx_remoto
                    && let Ok(mat) = rx.try_recv()
                    && let Some(color_image) = crate::vista::Vista::generar_imagen(&mat)
                {
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

                // Muestro la textura actualizada
                if let Some(tex) = &self.tex_remoto {
                    //ui.image((tex.id(), egui::vec2(ui.available_width() * 0.4, ui.available_height() * 0.5)));
                    ui.image((tex.id(), Self::redimensionar_textura(tex)));
                } else {
                    ui.heading("No Camera (Remoto)");
                }

                ui.add_space(20.0);

                // Actualizo frame local
                match self.cam_enable {
                    true => {
                        if let Some(rx) = &self.rx_local
                            && let Ok(mat) = rx.try_recv()
                            && let Some(color_image) = crate::vista::Vista::generar_imagen(&mat)
                        {
                            // Crear o reemplazar textura
                            if let Some(tex) = &mut self.tex_local {
                                tex.set(color_image, egui::TextureOptions::LINEAR);
                            } else {
                                self.tex_local = Some(ctx.load_texture(
                                    "cam_local",
                                    color_image,
                                    egui::TextureOptions::LINEAR,
                                ));
                            }
                        }

                        // Muestro la textura actualizada
                        if let Some(tex) = &self.tex_local {
                            ui.image((tex.id(), Self::redimensionar_textura(tex)));
                            //ui.image((tex.id(), egui::vec2(ui.available_width() * 0.4, ui.available_height() * 0.5)));
                        } else {
                            ui.heading("No Camera (Local)");
                        }
                    }
                    false => {
                        ui.heading("No Camera (Local)");

                        // Cargar la textura no_camera y posicionar acá
                    }
                }
            });
        });

        cambio
    }

    fn liberar_camara(&mut self) {
        self.cam_enable = false;
        // self.local_cam = None;
    }

    fn cargar_camara(&mut self) {
        self.cam_enable = true;
        // self.local_cam = Some(Camera::new().unwrap());
    }

    fn redimensionar_textura(tex: &TextureHandle) -> Vec2 {
        // Ajusto el tamaño manteniendo la relación 4:3
        let tex_size = tex.size_vec2();
        let max_size = egui::vec2(400.0, 300.0);

        let scale = (max_size.x / tex_size.x).min(max_size.y / tex_size.y);

        tex_size * scale
    }
}
