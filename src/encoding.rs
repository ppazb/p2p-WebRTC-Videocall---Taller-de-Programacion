extern crate opencv;
extern crate openh264;
use crate::encoding_errors::{DecodingErrors, EncodingErrors};
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::core::{MatTraitConst, MatTraitConstManual};
use opencv::imgproc;
use openh264::decoder::Decoder;
use openh264::encoder::{Encoder, EncoderConfig};
use openh264::formats::YUVSource;

pub struct EncoderRtc {
    encoderh264: Encoder,
    //encoderb:bool
}

impl EncoderRtc {
    pub fn new_encoding() -> Result<Self, EncodingErrors> {
        let config = EncoderConfig::new()
            .usage_type(openh264::encoder::UsageType::CameraVideoRealTime)
            .level(openh264::encoder::Level::Level_3_1)
            .skip_frames(true);

        let encoderh264 = Encoder::with_api_config(openh264::OpenH264API::from_source(), config)
            .map_err(|_| EncodingErrors::EncoderInit)?;

        Ok(Self {
            encoderh264
            // encoderb: false,
        })
    }

    

    pub fn encode(
        &mut self,
        frame: &opencv::core::Mat
    ) -> Result<Vec<u8>, EncodingErrors> {
        let mut yuv = opencv::core::Mat::default();
        imgproc::cvt_color(frame, &mut yuv, imgproc::COLOR_BGR2YUV_I420, 0)
            .map_err(|_| EncodingErrors::SetColorSpace)?;

        let width = frame.cols() as usize;
        let height = frame.rows() as usize;
        let yuv_bytes = yuv.data_bytes().map_err(|_| EncodingErrors::GetYuvBytes)?;
        let yuv_source = openh264::formats::YUVBuffer::from_vec(yuv_bytes.to_vec(), width, height);
        let encoded = self
            .encoderh264
            .encode(&yuv_source)
            .map_err(|_| EncodingErrors::EncodeYuv)?;
        Ok(encoded.to_vec())
    }
}

pub struct DecoderRtc {
    decoderh264: Decoder,
    //decoderb:bool
}

impl DecoderRtc {
    pub fn new_decoder() -> Result<Self, DecodingErrors> {
        let decoderh264 = Decoder::new().map_err(|_| DecodingErrors::DecodeInit)?;
        Ok(Self {
            decoderh264,
            //decoderb:false
        })
    }

    pub fn decode(
    &mut self,
    h264_data: &[u8],
    width: i32,
    height: i32,
    ) -> Result<opencv::core::Mat, DecodingErrors> {
        // Decode H264 data into YUV
        let yuv_decoded_opt = self.decoderh264
            .decode(h264_data)
            .map_err(|_| DecodingErrors::DecodeYuv)?;

        if let Some(yuv_decoded) = yuv_decoded_opt {
            println!("Decoded H264 -> YUV: Y={} U={} V={}", 
                    yuv_decoded.y().len(), 
                    yuv_decoded.u().len(), 
                    yuv_decoded.v().len());

            let mut yuv_buffer = Vec::with_capacity((width * height * 3 / 2) as usize);
            yuv_buffer.extend_from_slice(yuv_decoded.y());
            yuv_buffer.extend_from_slice(yuv_decoded.u());
            yuv_buffer.extend_from_slice(yuv_decoded.v());

            let mut yuv_frame = opencv::core::Mat::from_slice(&yuv_buffer)
                .map_err(|_| DecodingErrors::YuvFrameSettup)?;
            println!("YUV Mat created with rows={} cols={}", yuv_frame.rows(), yuv_frame.cols());


            let yuv_frame_ = yuv_frame
                .reshape(1, (height * 3 / 2) as i32)
                .map_err(|_| DecodingErrors::YuvFrameSettup)?;
            println!("YUV Mat reshaped to rows={} cols={}", yuv_frame_.rows(), yuv_frame_.cols());


            let mut bgr = opencv::core::Mat::default();
            imgproc::cvt_color(&yuv_frame_, &mut bgr, imgproc::COLOR_YUV2BGR_I420, 0)
                .map_err(|_| DecodingErrors::SetColorSpace)?;
            println!("Converted YUV -> BGR: rows={} cols={}", bgr.rows(), bgr.cols());

            Ok(bgr)
        } else {
            Err(DecodingErrors::DecodeYuv)
        }
    }

}
