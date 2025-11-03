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

    let screen_size = eframe::egui::Vec2::new(
        1366.0, // ancho
        768.0,  // alto
    );

    // con native_options se configuran distintos aspectos de la ventana
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(screen_size / 1.5)
            .with_min_inner_size(screen_size / 2.0),
        ..Default::default()
    };

    // genero la ventana de la aplicacion
    eframe::run_native(
        "RoomRTC - Sobrevivir Taller",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(AppUI::new(
                args[1].to_string(),
                args[2].to_string(),
            )))
        }),
    )
}
