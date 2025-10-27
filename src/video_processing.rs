extern crate opencv;
extern crate openh264;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::videoio::{self, VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst};
const CAMARA_WIDTH: f64 = 1280.0;
const CAMARA_HEIGHT: f64 = 720.0;
const CAMARA_FPS: f64 = 60.0;

// creo una captura de video y se configuran las propiedades
pub fn initialize_camara() -> Result<VideoCapture, VideoProcessingErrors> {
    let mut camara = match VideoCapture::new(0, videoio::CAP_V4L2) {
        Ok(camara) => camara,
        Err(_) => {
            return Err(VideoProcessingErrors::AccessCamara);
        }
    };
    camara_set_properties(&mut camara)?;
    Ok(camara)
}

//Settea las propiedades de la camara, toma las constantes. Estos valores actuan como limite superior
pub fn camara_set_properties(capture: &mut VideoCapture) -> Result<(), VideoProcessingErrors> {
    //Parameter 3 = CAP_PROP_FRAME_WIDTH
    capture
        .set(videoio::CAP_PROP_FRAME_WIDTH, CAMARA_WIDTH)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;

    //Parameter 4 = CAP_PROP_FRAME_HEIGHT
    capture
        .set(videoio::CAP_PROP_FRAME_HEIGHT, CAMARA_HEIGHT)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;

    capture
        .set(videoio::CAP_PROP_FPS, CAMARA_FPS)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;
    Ok(())
}

//Lee un frame de la camara y lo devuelve
pub fn read_frame(camara: &mut VideoCapture) -> Result<opencv::core::Mat, VideoProcessingErrors> {
    let mut frame = opencv::core::Mat::default();
    let _ = camara
        .read(&mut frame)
        .map_err(|_| VideoProcessingErrors::AccessCamara);
    Ok(frame)
}

//Obtiene el ancho del frame configurado en la camara
pub fn get_frame_width(camara: &VideoCapture) -> Result<i32, VideoProcessingErrors> {
    let width = camara
        .get(videoio::CAP_PROP_FRAME_WIDTH)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)? as i32;
    Ok(width)
}

//Obtiene el alto del frame configurado en la camara
pub fn get_frame_height(camara: &VideoCapture) -> Result<i32, VideoProcessingErrors> {
    let height = camara
        .get(videoio::CAP_PROP_FRAME_HEIGHT)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)? as i32;
    Ok(height)
}

//Tests
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    //camara se inicializa correctamente
    fn initialize_camara_test() {
        let camara = initialize_camara();
        assert!(camara.is_ok());
    }

    //No se agregan tests de las funcionalidades restantes, ya que no se mantienen constantes
    //entre equipo y equipo. A su vez, las funciones son wrappers de librerias externas que asumimos
    //correcto funcionamiento. Se agregan manejo de errores utilizando nuestras definiciones para mejor mappeo
}
