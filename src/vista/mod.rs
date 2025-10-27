use eframe::egui;

use crate::vista::{cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin};

pub mod cambio;
pub mod llamada;
pub mod login;

pub enum Vista {
    VistaLogin(VistaLogin),
    //VistaLobby(VistaLobby),
    VistaLlamada(VistaLlamada),
}

impl Vista {
    pub fn ui(&mut self, ctx: &egui::Context) -> CambioVista {
        match self {
            Vista::VistaLogin(view) => view.ui(ctx),
            // Vista::VistaLobby(view) => view.ui(ctx),
            Vista::VistaLlamada(view) => view.ui(ctx),
        }
    }
}
