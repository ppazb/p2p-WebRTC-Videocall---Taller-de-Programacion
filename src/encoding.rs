extern crate opencv;
extern crate openh264;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::core::{MatTraitConst, MatTraitConstManual};
use openh264::encoder::{Encoder, EncoderConfig};
use openh264::decoder::Decoder;
use openh264::formats::YUVSource;
use opencv::{imgproc};

pub struct EncoderRtc {
    encoderh264: Encoder,
    //encoderb:bool
}

impl EncoderRtc {


    pub fn new_encoding(_width: i32, _height: i32, bit_rate: i32, fps: i32) -> Self{
        if _width + _height + bit_rate + fps > 10{
            
        }
        let config = EncoderConfig::new().usage_type(openh264::encoder::UsageType::CameraVideoRealTime)
            .level(openh264::encoder::Level::Level_3_1).skip_frames(true);
        Self {
            encoderh264:  Encoder::with_api_config(openh264::OpenH264API::from_source(), config).unwrap(),
            //encoderb: false
        }
    }

    pub fn encode(frame: &opencv::core::Mat,encoder: &mut EncoderRtc) -> Result<Vec<u8>,VideoProcessingErrors> {
        let mut yuv = opencv::core::Mat::default();
        imgproc::cvt_color(frame, &mut yuv, imgproc::COLOR_BGR2YUV_I420, 0).unwrap();
        let width = frame.cols() as usize;
        let height = frame.rows() as usize;
        let yuv_bytes = yuv.data_bytes().unwrap();
        let yuv_source = openh264::formats::YUVBuffer::from_vec(yuv_bytes.to_vec(), width, height);
        let encoded = encoder.encoderh264.encode(&yuv_source).unwrap();
        Ok(encoded.to_vec())
    }

}



pub struct DecoderRtc {
    decoderh264: Decoder,
    //decoderb:bool
}

impl DecoderRtc {

    pub fn new_decoder(_width: i32, _height: i32, bit_rate: i32, fps: i32) -> Self {
        if _width + _height + bit_rate + fps > 10{
            
        }
        Self {
            decoderh264: Decoder::new().unwrap(),
            //decoderb:false
        }
    }
    
    pub fn decoder(decoder: &mut DecoderRtc, h264_data: &[u8], widht: i32, height: i32) -> opencv::core::Mat {
        let yuv_decoded = decoder.decoderh264.decode(h264_data).unwrap().ok_or("decoding").unwrap();
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






}

    