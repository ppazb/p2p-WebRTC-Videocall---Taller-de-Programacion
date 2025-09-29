use core::fmt;
use std::io;
//use std::str::FromStr;
//use std::str::SplitAsciiWhitespace;
pub enum ClientErrors {
    CantBind(io::Error),
    CantConect(io::Error),
    CantClone(io::Error),
    
}

impl From<io::Error> for ClientErrors  {
    fn from(err: io::Error) -> Self {
        Self::CantBind(err)
}
}

impl fmt::Display for ClientErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CantBind(_) => writeln!(f, "Error: \"No se pudo bindear\""),
            Self::CantConect(_) =>writeln!(f, "Error: \"No se pudo conectar\""),
            Self::CantClone(_) =>writeln!(f, "Error: \"No se pudo clonar el socket\"")
        }
    }
}


//----------------------------------------------Message error------------------------------------

pub enum MessageErrors {
    CantCreate,
    
}