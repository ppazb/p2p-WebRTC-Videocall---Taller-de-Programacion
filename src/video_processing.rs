extern crate opencv;
use crate::room_rtc_errors::VideoProcessingErrors;
use opencv::videoio::{self, VideoCapture, VideoCaptureTrait};

pub fn capture_frame() -> Result<opencv::core::Mat, VideoProcessingErrors> {
    let mut cam =
        VideoCapture::new(0, videoio::CAP_ANY).map_err(|_| VideoProcessingErrors::AccessCamara)?;

    let _ = camara_set_properties(&mut cam)?;

    let mut mat = opencv::core::Mat::default();
    cam.read(&mut mat).unwrap();
    Ok(mat)
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
