
use opencv::{prelude::*, videoio, highgui, error};

pub struct Camera{
    cam: videoio::VideoCapture,
}

impl Camera {
    pub fn new() -> Result<Self, error::Error> {
        let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?;
        // cam.set(videoio::CAP_PROP_FRAME_WIDTH, 640.0)?;
        // cam.set(videoio::CAP_PROP_FRAME_HEIGHT, 480.0)?;

        Ok(Self {cam})
    }

    pub fn leer_frame(&mut self) -> Mat {
        // Tipo matricial para almacenar información de imagen
        let mut frame = Mat::default();
        let _ = self.cam.read(&mut frame);

        frame
    }
}

fn main() -> opencv::Result<()> {
    // Elemento Camara, dispositivo default y API de captura indistinta
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?;
    
    // Ventana externa para mostrar video
    highgui::named_window("Test", highgui::WINDOW_FULLSCREEN)?;

    // Tipo matricial para almacenar información de imagen
    let mut frame = Mat::default();

    loop {
        // Leo estado de cámara, cargo datos en frame
        cam.read(&mut frame)?;

        // Cargo frame en la ventana
        highgui::imshow("Test", &frame)?;

        // Ventana espera milisegundos a que se presione una tecla para salir
        let key = highgui::wait_key(1000)?;
        if key == 'q' as i32 { break; }
    }
    
    
    Ok(())
}
