use crate::tls_error::TlsError;
use std::fmt;

#[derive(Debug, Clone)]
pub enum SignallingServerError {
    ConnectionError,
    ConfigInfoError,
    TlsHandshakeError(TlsError),
}

impl fmt::Display for SignallingServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError => write!(f, "Error: \"Connection error\""),
            Self::ConfigInfoError => write!(f, "Error: \"Config information error\""),
            Self::TlsHandshakeError(e) => write!(f, "Error: \"TLS: {:?}\"", e),
        }
    }
}
