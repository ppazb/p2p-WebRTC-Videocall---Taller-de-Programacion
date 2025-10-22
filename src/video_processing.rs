extern crate opencv;
extern crate openh264;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::highgui;
use opencv::videoio::{self, VideoCapture, VideoCaptureTrait, VideoCaptureTraitConst};

pub fn initialize_camara() -> Result<VideoCapture, VideoProcessingErrors> {
    let mut camara = match VideoCapture::new(0, videoio::CAP_V4L2) {
        Ok(camara) => camara,
        Err(e) => {
            println!("hubo error!!!!! {}", e);
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
    println!("Estoy tomando: {}x{}", frame_w, frame_h);
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
/*
pub fn encode(frame: &opencv::core::Mat,encoder: &mut Encoder) -> Result<Vec<u8>,VideoProcessingErrors> {
    let mut yuv = opencv::core::Mat::default();
    imgproc::cvt_color(frame, &mut yuv, imgproc::COLOR_BGR2YUV_I420, 0).unwrap();
    let width = frame.cols() as usize;
    let height = frame.rows() as usize;
    let yuv_bytes = yuv.data_bytes().unwrap();
    let yuv_source = openh264::formats::YUVBuffer::from_vec(yuv_bytes.to_vec(), width, height);
    let encoded = encoder.encode(&yuv_source).unwrap();
    Ok(encoded.to_vec())
}

pub fn decoder(decoder: &mut Decoder, h264_data: &[u8], widht: i32, height: i32) -> opencv::core::Mat {
    let yuv_decoded = decoder.decode(h264_data).unwrap().ok_or("decoding").unwrap();
    let mut yuv_buffer = Vec::with_capacity( (widht * height * 3 / 2 )as usize);
    yuv_buffer.extend_from_slice(yuv_decoded.y());
    yuv_buffer.extend_from_slice(yuv_decoded.u());
    yuv_buffer.extend_from_slice(yuv_decoded.v());

    let yuv_frame = opencv::core::Mat::from_slice(&yuv_buffer).unwrap();
    let yuv_frame = yuv_frame.reshape(1,height * 3/2).unwrap();

    let mut bgr = opencv::core::Mat::default();

    imgproc::cvt_color(&yuv_frame, &mut bgr, imgproc::COLOR_YUV2BGR_I420, 0).unwrap();

    bgr
}

pub fn create_encoder(width: i32, height: i32, bit_rate: i32, fps: i32) -> encoder::Encoder {
    let encoder_h264 = Encoder::new().unwrap();
    encoder_h264
}


pub fn create_decoder(width: i32, height: i32, bit_rate: i32, fps: i32) -> decoder::Decoder {
    let decoder_h264 = Decoder::new().unwrap();
    decoder_h264
}
*/
