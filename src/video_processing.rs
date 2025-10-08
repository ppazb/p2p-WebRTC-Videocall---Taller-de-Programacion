extern crate opencv;
use crate::room_rtc_errors::VideoProcessingErrors;
use opencv::highgui;
use opencv::videoio::{self, VideoCapture, VideoCaptureTrait};

pub fn initialize_camara() -> Result<VideoCapture, VideoProcessingErrors> {
    let mut camara: VideoCapture =
        VideoCapture::new(0, videoio::CAP_ANY).map_err(|_| VideoProcessingErrors::AccessCamara)?;

    camara_set_properties(&mut camara)?;
    Ok(camara)
}

pub fn camara_set_properties(capture: &mut VideoCapture) -> Result<(), VideoProcessingErrors> {
    //Parameter 3 = CAP_PROP_FRAME_WIDTH
    capture
        .set(3, 640.0)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;

    //Parameter 4 = CAP_PROP_FRAME_HEIGHT
    capture
        .set(4, 480.0)
        .map_err(|_| VideoProcessingErrors::CamaraSettup)?;
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
