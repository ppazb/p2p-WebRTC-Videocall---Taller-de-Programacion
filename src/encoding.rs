extern crate opencv;
extern crate openh264;
use crate::encoding_errors::{DecodingErrors, EncodingErrors};
use opencv::core::{MatTraitConst, MatTraitConstManual};
use opencv::imgproc;
use openh264::decoder::Decoder;
use openh264::encoder::{BitRate, Encoder, EncoderConfig};
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
            .skip_frames(true).bitrate(BitRate::from_bps(3000));

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
        self.encoderh264.force_intra_frame();
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
        widht: i32,
        height: i32,
    ) -> Result< opencv::core::Mat, DecodingErrors> {
        if let Some(yuv_decoded) = self.decoderh264.decode(h264_data).map_err(|_| DecodingErrors::DecodeYuv)? {
        let mut yuv_buffer = Vec::with_capacity((widht * height * 3 / 2) as usize);
        yuv_buffer.extend_from_slice(yuv_decoded.y());
        yuv_buffer.extend_from_slice(yuv_decoded.u());
        yuv_buffer.extend_from_slice(yuv_decoded.v());

        let yuv_frame = opencv::core::Mat::from_slice(&yuv_buffer)
            .map_err(|_| DecodingErrors::YuvFrameSettup)?;
        let yuv_frame = yuv_frame
            .reshape(1, height * 3 / 2)
            .map_err(|_| DecodingErrors::YuvFrameSettup)?;

        let mut bgr = opencv::core::Mat::default();

        imgproc::cvt_color(&yuv_frame, &mut bgr, imgproc::COLOR_YUV2BGR_I420, 0)
            .map_err(|_| DecodingErrors::SetColorSpace)?;

        Ok(bgr)
        }
        else {
            Err(DecodingErrors::DecodeYuv)
        }    
    }

}
