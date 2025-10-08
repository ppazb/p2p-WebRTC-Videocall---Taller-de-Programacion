use std::fmt;

#[derive(Debug)]
pub enum SdpError {
    ParseError,
    ParseIntError(std::num::ParseIntError),
    InvalidOriginFormat,
    InvalidTimeFormat,
    InvalidMediaFormat,
    InvalidadAdder,
    MissingField(String),
    OfferFileCreationError,
    OfferFileWriteError,
}

impl From<std::num::ParseIntError> for SdpError {
    fn from(err: std::num::ParseIntError) -> Self {
        SdpError::ParseIntError(err)
    }
}

impl fmt::Display for SdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError => write!(f, "Error: \"Parse error .sdp\""),
            Self::ParseIntError(e) => write!(f, "Error: \"Parse int error .sdp: {}\"", e),
            Self::InvalidOriginFormat => write!(f, "Error: \"Formato o (origin) inválido\""),
            Self::InvalidTimeFormat => write!(f, "Error: \"Formato t (Time) inválido\""),
            Self::InvalidMediaFormat => write!(f, "Error: \"Formato m (Media) inválido\""),
            Self::MissingField(field) => write!(f, "Error: \"Campo {} no especificado\"", field),
            Self::InvalidadAdder => write!(f, "Error: \"Invalid address\""),
            Self::OfferFileCreationError => write!(f, "Error: \"Can not generate offer file\""),
            Self::OfferFileWriteError => write!(f, "Error: \"Can not write in offer file\""),
        }
    }
}
