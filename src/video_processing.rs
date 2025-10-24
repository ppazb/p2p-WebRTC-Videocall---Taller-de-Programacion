extern crate opencv;
extern crate openh264;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::highgui;
use opencv::videoio::{self, VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst};

pub fn initialize_camara() -> Result<VideoCapture, VideoProcessingErrors> {
    let mut camara = match VideoCapture::new(0, videoio::CAP_V4L2) {
        Ok(camara) => camara,
        Err(e) => {
            return Err(VideoProcessingErrors::AccessCamara);
        }
    };
    camara_set_properties(&mut camara)?;
    Ok(camara)
}

pub fn camara_set_properties(capture: &mut VideoCapture) -> Result<(), VideoProcessingErrors> {
    //Parameter 3 = CAP_PROP_FRAME_WIDTH
    capture
        .set(videoio::CAP_PROP_FRAME_WIDTH, 1280.0)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;

    //Parameter 4 = CAP_PROP_FRAME_HEIGHT
    capture
        .set(videoio::CAP_PROP_FRAME_HEIGHT, 720.0)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;

    capture
        .set(videoio::CAP_PROP_FPS, 30.0)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;

    //temporal
    let frame_w = get_frame_width(capture).unwrap();
    let frame_h = get_frame_height(capture).unwrap();
    Ok(())
}

pub fn display_frame(frame: &opencv::core::Mat) -> Result<(), VideoProcessingErrors> {
    // Display frame
    highgui::imshow("Video", &frame).map_err(|_| VideoProcessingErrors::DisplayError)?;
    Ok(())
}

pub fn read_frame(camara: &mut VideoCapture) -> Result<opencv::core::Mat, VideoProcessingErrors> {
    let mut frame = opencv::core::Mat::default();
    let _ = camara
        .read(&mut frame)
        .map_err(|_| VideoProcessingErrors::AccessCamara);
    Ok(frame)
}

pub fn get_frame_width(camara: &VideoCapture) -> Result<i32, VideoProcessingErrors> {
    let width = camara
        .get(videoio::CAP_PROP_FRAME_WIDTH)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)? as i32;
    Ok(width)
}

pub fn get_frame_height(camara: &VideoCapture) -> Result<i32, VideoProcessingErrors> {
    let height = camara
        .get(videoio::CAP_PROP_FRAME_HEIGHT)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)? as i32;
    Ok(height)
}