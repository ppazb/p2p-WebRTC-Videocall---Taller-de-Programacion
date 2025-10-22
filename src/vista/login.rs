use eframe::egui;

use crate::vista::cambio::CambioVista;

pub struct VistaLogin {
    pub username: String,
    pub password: String,
    pub error_msg: Option<String>,
}

impl VistaLogin {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            error_msg: None,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista{
        let mut cambio = CambioVista::None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Login");

            ui.text_edit_singleline(&mut self.username);
            ui.text_edit_singleline(&mut self.password);

            if ui.button("Login").clicked() {
                if self.username == "admin" && self.password == "1234" {
                    cambio = CambioVista::Llamada;
                } else {
                    self.password = String::new();
                    self.error_msg = Some("Usuario o contraseña inválidos.".into());
                }
            }

            if let Some(msg) = &self.error_msg {
                ui.colored_label(egui::Color32::RED, msg);
            }
        });

        return cambio;
    }
}
