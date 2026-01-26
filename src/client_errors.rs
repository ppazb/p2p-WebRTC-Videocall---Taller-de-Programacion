use crate::encoding_errors::{DecodingErrors, EncodingErrors};
use crate::ice_errors::IceErrors;
use crate::mensaje_errors::MessageErrors;
use crate::sctp_error::SctpError;
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
    SctpError(SctpError), 
    SdpError(SdpError),
    IceError(IceErrors),
    SocketNotInitialized,
    StatsCouldtGets,
}

impl From<io::Error> for ClientErrors {
    fn from(error: io::Error) -> Self {
        Self::CantBind(error)
    }
}

impl From<VideoProcessingErrors> for ClientErrors {
    fn from(error: VideoProcessingErrors) -> Self {
        Self::VideoProcessingError(error)
    }
}

impl From<MessageErrors> for ClientErrors {
    fn from(error: MessageErrors) -> Self {
        Self::MessageError(error)
    }
}

impl From<EncodingErrors> for ClientErrors {
    fn from(error: EncodingErrors) -> Self {
        Self::EncodingError(error)
    }
}

impl From<DecodingErrors> for ClientErrors {
    fn from(error: DecodingErrors) -> Self {
        Self::DecodingError(error)
    }
}

impl From<SctpError> for ClientErrors {
    fn from(error: SctpError) -> Self {
        ClientErrors::SctpError(error)
    }
}

impl From<SdpError> for ClientErrors {
    fn from(error: SdpError) -> Self {
        ClientErrors::SdpError(error)
    }
}

impl From<IceErrors> for ClientErrors {
    fn from(error: IceErrors) -> Self {
        ClientErrors::IceError(error)
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
            Self::SctpError(sctp_error) => writeln!(f, "{}", sctp_error),
            Self::SdpError(sdp_error) => writeln!(f, "{}", sdp_error),
            Self::IceError(ice_error) => writeln!(f, "{}", ice_error),
            Self::SocketNotInitialized => writeln!(f, "El cliente no tiene socket"),
            Self::StatsCouldtGets => writeln!(f, "No se pudo obtener las estadisticas"),
        }
    }
}
