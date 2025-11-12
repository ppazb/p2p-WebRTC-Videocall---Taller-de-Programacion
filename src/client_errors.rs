use dimpl::certificate::{CertificateError};

use crate::encoding_errors::{DecodingErrors, EncodingErrors};
use crate::ice_errors::IceErrors;
use crate::mensaje_errors::MessageErrors;
use crate::sdp_errors::SdpError;
use crate::video_processing_errors::VideoProcessingErrors;
use core::fmt;
use std::io;
///Se generar todos los errores para cliente y se adjuntan las conversiones de errores de otros crates
#[derive(Debug)]
pub enum ClientErrors {
    CantBind(io::Error),
    CantConect(io::Error),
    CantClone(io::Error),
    CantSendMsg(io::Error),
    MessageError(MessageErrors),
    VideoProcessingError(VideoProcessingErrors),
    EncodingError(EncodingErrors),
    DecodingError(DecodingErrors),
    SerializeBits,
    SendBits,
    SdpError(SdpError),
    IceError(IceErrors),
    DtlsError(CertificateError),
    SocketNotInitialized
}

impl From<io::Error> for ClientErrors {
    fn from(err: io::Error) -> Self {
        Self::CantBind(err)
    }
}

impl From<VideoProcessingErrors> for ClientErrors {
    fn from(err: VideoProcessingErrors) -> Self {
        Self::VideoProcessingError(err)
    }
}

impl From<MessageErrors> for ClientErrors {
    fn from(err: MessageErrors) -> Self {
        Self::MessageError(err)
    }
}

impl From<EncodingErrors> for ClientErrors {
    fn from(err: EncodingErrors) -> Self {
        Self::EncodingError(err)
    }
}

impl From<DecodingErrors> for ClientErrors {
    fn from(err: DecodingErrors) -> Self {
        Self::DecodingError(err)
    }
}
impl From<CertificateError> for ClientErrors {
    fn from(err: CertificateError) -> Self {
        Self::DtlsError(err)
    }
}
impl fmt::Display for ClientErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CantBind(_) => writeln!(f, "Error: \"No se pudo bindear\""),
            Self::CantConect(_) => writeln!(f, "Error: \"No se pudo conectar\""),
            Self::CantClone(_) => writeln!(f, "Error: \"No se pudo clonar el socket\""),
            Self::CantSendMsg(_) => writeln!(f, "Error: \"No se pudo enviar el mensaje\""),
            Self::MessageError(message_error) => writeln!(f, "{}", message_error),
            Self::VideoProcessingError(message_error) => writeln!(f, "{}", message_error),
            Self::EncodingError(message_error) => writeln!(f, "{}", message_error),
            Self::DecodingError(message_error) => writeln!(f, "{}", message_error),
            Self::SerializeBits => writeln!(f, "Error: \"No se pudo serializar los bits\""),
            Self::SendBits => writeln!(f, "Error: \"No se pudieron enviar los bits\""),
            Self::SdpError(sdp_error) => writeln!(f, "{}", sdp_error),
            Self::IceError(ice_error) => writeln!(f, "{}", ice_error),
            Self::DtlsError(dtls_error) => writeln!(f, "{}", dtls_error),
            Self::SocketNotInitialized  => writeln!(f, "El cliente no tiene socket")
        }
    }
}
