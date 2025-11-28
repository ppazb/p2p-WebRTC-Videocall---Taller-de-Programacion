use std::fmt;

#[derive(Debug, Clone)]
pub enum TlsError {
    PemfileError,
    AbsentPrivateKeyError,
    IpParseError,
    ClientConnectionError,
    ServerConfigError,
    HandshakeError, 
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsError::PemfileError => write!(f, "[TLS] Error: \".pem file error\""),
            TlsError::AbsentPrivateKeyError => write!(f, "[TLS] Error: \"No private key found\""),
            TlsError::IpParseError => write!(f, "[TLS] Error: \"IP parse error\""),
            TlsError::ClientConnectionError => write!(f, "[TLS] Error: \"ClientConnection error\""),
            TlsError::ServerConfigError => write!(f, "[TLS] Error: \"ServerConfig error\""),
            TlsError::HandshakeError => write!(f, "[TLS] Error: \"Handshake error\""),
        }
    }
}