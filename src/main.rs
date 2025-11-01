use eframe::egui;

use room_rtc::app::AppUI;

fn main() -> eframe::Result<()> {
    // DEBUG: asignar los puertos desde consola
    // cargo run --bin sdp_offer <ip:port> <ip:port>
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Indicar los puertos local y remoto");
        return Ok(());
    }

    // con native_options se configuran distintos aspectos de la ventana
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            //.with_min_inner_size([850.0, 600.0]),
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    // genero la ventana de la aplicacion
    eframe::run_native(
        "Prueba eGUI",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(AppUI::new(
                args[1].to_string(),
                args[2].to_string(),
            )))
        }),
    )
}
