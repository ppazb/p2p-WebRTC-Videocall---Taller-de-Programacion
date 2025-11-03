//----------------------------------------------Camara error------------------------------------
use core::fmt;

#[derive(Debug)]
pub enum VideoProcessingErrors {
    AccessCamara,
    CamaraSettup,
    DisplayError,
}

impl fmt::Display for VideoProcessingErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessCamara => writeln!(f, r#"Error: "No se pudo conectar a la cámara"#),
            Self::CamaraSettup => writeln!(f, r#"Error: "No se pudo configurar a la cámara"#),
            Self::DisplayError => writeln!(f, r#"Error: "No se pudo mostrar la cámara"#),
        }
    }
}
