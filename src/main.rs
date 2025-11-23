use eframe::egui;

use room_rtc::{app::AppUI, config::get_config_app};

fn main() -> eframe::Result<()> {
    // DEBUG: asignar los puertos desde consola
    // cargo run --bin sdp_offer <ip:port> <ip:port>
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Use: {} <file_path_config>", args[0]);
        return Ok(());
    }

    // Cargar parámetros de configuración (IP servidor, puertos locales, etc.)
    let session_info = match get_config_app(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprint!("Error in config: {:?}", e);
            return Ok(());
        }
    };

    let screen_size = eframe::egui::Vec2::new(
        1366.0, // ancho
        768.0,  // alto
    );

    // con native_options se configuran distintos aspectos de la ventana
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(screen_size / 1.5)
            .with_min_inner_size(screen_size / 2.0)
            .with_title("RoomRTC - Cliente"),
        ..Default::default()
    };

    let app = match AppUI::new(session_info) {
        Some(app) => app,
        None => {
            return eframe::Result::Err(eframe::Error::AppCreation(
                "Failing to initialize the app: cant connect to signaling server".into(),
            ));
        }
    };

    eframe::run_native("RoomRTC", native_options, Box::new(|_cc| Ok(Box::new(app))))
}
