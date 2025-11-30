use core::fmt;

#[derive(Debug)]
pub enum IceErrors {
    NoValidCodec,
    FailedConnection,
    StunError,
}

impl fmt::Display for IceErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoValidCodec => writeln!(
                f,
                r#"Error: "No se encontró un códec válido para la conexión ICE"#
            ),
            Self::FailedConnection => writeln!(f, r#"Error: "La conexión ICE falló"#),
            Self::StunError => writeln!(f, r#"Error: "Error en STUN"#),
        }
    }
}

impl From<std::io::Error> for IceErrors {
    fn from(err: std::io::Error) -> Self {
        eprintln!("[ICE ERROR] IO Error: {:?}", err);
        IceErrors::StunError
    }
}
