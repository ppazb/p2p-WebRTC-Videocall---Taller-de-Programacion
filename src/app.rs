use eframe::egui;
use crate::vista::{cambio::CambioVista, llamada::VistaLlamada, login::VistaLogin, Vista};

pub struct AppUI {
    vista_actual: Vista,
}

impl AppUI {
    pub fn new() -> Self {
        Self {
            vista_actual: Vista::VistaLlamada(VistaLlamada::new())
        }
    }
}

impl eframe::App for AppUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::light()); // o Visuals::light()

        // cada frame, la vista actual muestra su ui
        // algunas acciones provocan cambios de vista
        // se debería preservar información y cargarla durante la transicion
        let cambio = self.vista_actual.ui(ctx);
        
        match cambio {
            CambioVista::Login => self.vista_actual = Vista::VistaLogin(VistaLogin::new()),
            CambioVista::None => {},
            CambioVista::Llamada => self.vista_actual = Vista::VistaLlamada(VistaLlamada::new()),
        }
    }
}
