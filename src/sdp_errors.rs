use std::fmt;

#[derive(Debug, Clone)]
pub enum SdpError {
    ConnectionError,
    ParseError,
    ParseIntError(std::num::ParseIntError),
    InvalidOriginFormat,
    InvalidTimeFormat,
    InvalidMediaFormat,
    InvalidAdder,
    MissingField(String),
    MissingIceFields,
    OfferFileCreationError,
    OfferFileWriteError,
    AnswerFileCreationError,
    IncompatibleSession,
    CodecExtractionError,
    Other(String),
}

impl From<std::num::ParseIntError> for SdpError {
    fn from(err: std::num::ParseIntError) -> Self {
        SdpError::ParseIntError(err)
    }
}

impl fmt::Display for SdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError => write!(f, "Error: \"Connection error\""),
            Self::ParseError => write!(f, "Error: \"Parse error .sdp\""),
            Self::ParseIntError(e) => write!(f, "Error: \"Parse int error .sdp: {}\"", e),
            Self::InvalidOriginFormat => write!(f, "Error: \"Formato o (origin) inválido\""),
            Self::InvalidTimeFormat => write!(f, "Error: \"Formato t (Time) inválido\""),
            Self::InvalidMediaFormat => write!(f, "Error: \"Formato m (Media) inválido\""),
            Self::MissingField(field) => write!(f, "Error: \"Campo {} no especificado\"", field),
            Self::MissingIceFields => write!(f, "Error: \"Campo ICE no especificado\""),
            Self::InvalidAdder => write!(f, "Error: \"Direccion inválida\""),
            Self::OfferFileCreationError => write!(f, "Error: \"No se pudo crear offer file\""),
            Self::OfferFileWriteError => write!(f, "Error: \"No se pude escribir offer file\""),
            Self::AnswerFileCreationError => write!(f, "Error: \"No se pude crear answer file\""),
            Self::IncompatibleSession => write!(f, "Error: \"Sesión incompatible\""),
            Self::CodecExtractionError => write!(f, "Error: \"Error getting codec\""),
            Self::Other(message) => write!(f, "Error: \"{}\"", message),
        }
    }
}
