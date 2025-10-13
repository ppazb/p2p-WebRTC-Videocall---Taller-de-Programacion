use core::fmt;
use std::io;
use crate::mensaje_errors::MessageErrors;
use crate::video_processing_errors::VideoProcessingErrors;
//use std::str::FromStr;
//use std::str::SplitAsciiWhitespace;
#[derive(Debug)]
pub enum ClientErrors {
    CantBind(io::Error),
    CantConect(io::Error),
    CantClone(io::Error),
    CantSendMsg(io::Error),
    MessageError(MessageErrors),
    VideoProcessingError(VideoProcessingErrors),
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

impl fmt::Display for ClientErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CantBind(_) => writeln!(f, "Error: \"No se pudo bindear\""),
            Self::CantConect(_) => writeln!(f, "Error: \"No se pudo conectar\""),
            Self::CantClone(_) => writeln!(f, "Error: \"No se pudo clonar el socket\""),
            Self::CantSendMsg(_) => writeln!(f, "Error: \"No se pudo enviar el mensaje\""),
            Self::MessageError(message_error) => writeln!(f, "{}", message_error),
            Self::VideoProcessingError(message_error) => writeln!(f, "{}", message_error),
        }
    }
}

