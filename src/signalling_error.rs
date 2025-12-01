use crate::server_users_errors::LoginErrors;
use crate::tls_error::TlsError;
use std::fmt;

#[derive(Debug, Clone)]
pub enum SignallingServerError {
    ConnectionError,
    ConfigInfoError,
    TlsHandshakeError(TlsError),
    FileNotFound(LoginErrors),
    SyncError,
}

impl fmt::Display for SignallingServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError => write!(f, "Error: \"Connection error\""),
            Self::ConfigInfoError => write!(f, "Error: \"Config information error\""),
            Self::TlsHandshakeError(e) => write!(f, "Error: \"TLS: {:?}\"", e),
            Self::FileNotFound(e) => write!(f, "Error: \"USER DATA: {:?}\"", e),
            Self::SyncError => write!(f, "Error: \"Cant use resource right now\""),
        }
    }
}
