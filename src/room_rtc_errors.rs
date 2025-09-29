use core::fmt;
use std::io;
//use std::str::FromStr;
//use std::str::SplitAsciiWhitespace;
pub enum ClientErrors {
    CantBind(io::Error),
    CantConect(io::Error),
    CantClone(io::Error),
    CantSendMsg(io::Error),
    From(MessageErrors),
}

impl From<io::Error> for ClientErrors {
    fn from(err: io::Error) -> Self {
        Self::CantBind(err)
    }
}

impl fmt::Display for ClientErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CantBind(_) => writeln!(f, "Error: \"No se pudo bindear\""),
            Self::CantConect(_) => writeln!(f, "Error: \"No se pudo conectar\""),
            Self::CantClone(_) => writeln!(f, "Error: \"No se pudo clonar el socket\""),
            Self::CantSendMsg(_) => writeln!(f, "Error: \"No se pudo enviar el mensaje\""),
            Self::From(message_error) => writeln!(f, "{}", message_error),
        }
    }
}

//----------------------------------------------Message error------------------------------------

pub enum MessageErrors {
    CantCreate,
    MsgTooLarge,
}

impl fmt::Display for MessageErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CantCreate => writeln!(f, "Error: \"No se pudo crear el mensaje\""),
            Self::MsgTooLarge => writeln!(
                f,
                "Error: \"No se pudo enviar el mensaje, mesaje muy grande\""
            ),
        }
    }
}
