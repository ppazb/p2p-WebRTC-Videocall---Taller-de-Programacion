use eframe::egui;

use crate::app::AppUI;

pub mod vista;
mod app;
pub mod camera;

fn main() -> eframe::Result<()> {
    // con native_options se configuran distintos aspectos de la ventana
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
                        //.with_inner_size([800.0, 600.0])
                        .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    // genero la ventana de la aplicacion
    eframe::run_native(
        "Prueba eGUI", 
        native_options, 
        Box::new(|_cc| Ok(Box::new(AppUI::new()))))
}