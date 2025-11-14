use std::fmt;

#[derive(Debug, Clone)]
pub enum SignallingServerError {
    ConnectionError,
    ConfigInfoError,
}

impl fmt::Display for SignallingServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError => write!(f, "Error: \"Connection error\""),
            Self::ConfigInfoError => write!(f, "Error: \"Config information error\""),
        }
    }
}
