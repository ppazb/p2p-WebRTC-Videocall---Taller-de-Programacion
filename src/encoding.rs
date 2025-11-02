extern crate opencv;
extern crate openh264;
use crate::encoding_errors::{DecodingErrors, EncodingErrors};
use opencv::core::{MatTraitConst, MatTraitConstManual};
use opencv::imgproc;
use openh264::decoder::Decoder;
use openh264::encoder::{Encoder, EncoderConfig};
use openh264::formats::YUVSource;

///Abstraccion del encodear y decoder de openh264 para nuestro uso custom
pub struct EncoderRtc {
    encoderh264: Encoder,
}

impl EncoderRtc {
    /// Constructor del encoder configurado para el mejor rendimiento en videollamada tiempo real
    /// segun documentacion openh264
    pub fn new_encoding() -> Result<Self, EncodingErrors> {
        let config = EncoderConfig::new()
            .usage_type(openh264::encoder::UsageType::CameraVideoRealTime)
            .level(openh264::encoder::Level::Level_3_1)
            .skip_frames(true);

        let encoderh264 = Encoder::with_api_config(openh264::OpenH264API::from_source(), config)
            .map_err(|_| EncodingErrors::EncoderInit)?;

        Ok(Self {
            encoderh264
        })
    }
    /// funcion del encoder para encodear frames, recivimos un frame tipico de opcenCV el cual debe ser convertido
    /// a formato YUV Source para poder encodearlo con OPENH264, por lo tanto se recive el frame y se convierte a formato
    /// YUV y luego se encodea, en caso de funciona devuelve un frame encodeado, sino levanta error.
    pub fn encode(&mut self, frame: &opencv::core::Mat) -> Result<Vec<u8>, EncodingErrors> {
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
}

impl DecoderRtc {
    ///constructor para decoder
    pub fn new_decoder() -> Result<Self, DecodingErrors> {
        let decoderh264 = Decoder::new().map_err(|_| DecodingErrors::DecodeInit)?;
        Ok(Self {
            decoderh264,
        })
    }
    /// funcion de decoding en la cual recivimos una tira de bytes, el cual primero se deserializa al frame
    /// YUV source y luego si esto funciona es convertido nuevamente a un formato compatible con OPENCV.
    /// Es necesario tener el ancho y alto en que se encodo originalment el mesaje porque pueden ser camara con calidades
    /// distintas, por lo cual no se puede asumir sus dimensiones. El frames es recosntruido dimension a dimnension. 
    /// el ciclo es bytes -> YUV -> BGR si el proceso funciona correctamente devuelve el frame en formato compatible
    /// con OPENCV. 
    pub fn decode(
        &mut self,
        h264_data: &[u8],
        widht: i32,
        height: i32,
    ) -> Result<opencv::core::Mat, DecodingErrors> {
        if let Some(yuv_decoded) = self
            .decoderh264
            .decode(h264_data)
            .map_err(|_| DecodingErrors::DecodeYuv)?
        {
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
        } else {
            Err(DecodingErrors::DecodeYuv)
        }
    }
}
