use opencv::core::Mat;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{video_processing, video_processing_errors::VideoProcessingErrors};

pub struct Camera {
    is_active: Arc<AtomicBool>,
    last_frame: Arc<Mutex<Option<Mat>>>,
    handle: Option<JoinHandle<Result<(), VideoProcessingErrors>>>,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            last_frame: Arc::new(Mutex::new(None)),
            handle: None,
        }
    }

    pub fn enable_camera(&mut self) {
        if self.is_active.load(Ordering::Relaxed) {
            return;
        }

        // Clono los datos que necesito dentro del thread
        self.is_active.store(true, Ordering::Relaxed);

        let is_active = Arc::clone(&self.is_active);
        let last_frame = Arc::clone(&self.last_frame);

        let handle = thread::spawn(move || -> Result<(), VideoProcessingErrors> {
            let mut cam = match video_processing::initialize_camara() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error iniciando cámara: {:?}", e);
                    return Ok(());
                }
            };

            // Loop de captura mientras la cámara esté habilitada
            while is_active.load(Ordering::Relaxed) {
                match video_processing::read_frame(&mut cam) {
                    Ok(frame) => {
                        if let Ok(mut f) = last_frame.lock() {
                            *f = Some(frame);
                        }
                    }
                    Err(e) => {
                        eprintln!("CAMARA: Error leyendo frame: {:?}", e);
                    }
                }

                // Limita la frecuencia de lectura de cámara (30 FPS aprox)
                thread::sleep(Duration::from_millis(33));
            }

            Ok(())
        });

        self.handle = Some(handle);
    }

    pub fn disable_camera(&mut self) {
        if !self.is_active.load(Ordering::Relaxed) {
            return;
        }

        // Seteo flag para finalizar el thread de captura
        self.is_active.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        // Desreferencio el último frame guardado
        if let Ok(mut f) = self.last_frame.lock() {
            *f = None;
        }
    }

    pub fn get_last_frame(&self) -> Option<Mat> {
        if let Ok(f) = self.last_frame.lock() {
            f.clone()
        } else {
            None
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        self.disable_camera();
    }
}
