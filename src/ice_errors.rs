use core::fmt;

#[derive(Debug)]
pub enum IceErrors {
    NoValidCodec,
    FailedConnection,
}

impl fmt::Display for IceErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoValidCodec => writeln!(
                f,
                r#"Error: "No se encontró un códec válido para la conexión ICE"#
            ),
            Self::FailedConnection => writeln!(f, r#"Error: "La conexión ICE falló"#),
        }
    }
}
