use eframe::egui;

use crate::vista::cambio::CambioVista;

pub struct VistaSendFile {
    file_path: String,
    error_message: Option<String>,
}

impl Default for VistaSendFile {
    fn default() -> Self {
        Self::new()
    }
}

impl VistaSendFile {
    pub fn new() -> Self {
        Self {
            file_path: String::new(),
            error_message: None,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista {
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Enviar archivo");
            ui.add_space(8.0);

            ui.label("Pegá o escribí la ruta del archivo:");
            ui.text_edit_singleline(&mut self.file_path);

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Enviar").clicked() {
                    let p = std::path::Path::new(&self.file_path);
                    if !p.exists() {
                        self.error_message = Some("La ruta no existe.".into());
                    } else if !p.is_file() {
                        self.error_message = Some("La ruta no es un archivo.".into());
                    } else {
                        self.error_message = None;
                        cambio = CambioVista::SendFileConfirmed(self.file_path.clone());
                    }
                }

                if ui.button("Volver").clicked() {
                    self.error_message = None;
                    cambio = CambioVista::SendFileQuit;
                }
            });

            if let Some(err) = &self.error_message {
                ui.add_space(6.0);
                ui.colored_label(egui::Color32::RED, err);
            }
        });

        cambio
    }
}
