use core::fmt;

#[derive(Debug)]
pub enum EncodingErrors {
    SetColorSpace,
    EncoderInit,
    GetYuvBytes,
    EncodeYuv,
}
///Todas las conversiones y manejo de error para encodeo y decodeo
impl fmt::Display for EncodingErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetColorSpace => {
                writeln!(f, r#"Error: "No se pudo convertir el espacio de color"#)
            }
            Self::EncoderInit => writeln!(f, r#"Error: "No se pudo inicializar el codificador"#),
            Self::GetYuvBytes => writeln!(f, r#"Error: "No se pudieron obtener los bytes YUV"#),
            Self::EncodeYuv => writeln!(f, r#"Error: "No se pudo codificar el frame YUV"#),
        }
    }
}

#[derive(Debug)]
pub enum DecodingErrors {
    DecodeInit,
    DecodeYuv,
    YuvFrameSettup,
    SetColorSpace,
}

impl fmt::Display for DecodingErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecodeInit => writeln!(f, r#"Error: "No se pudo inicializar el decodificador"#),
            Self::DecodeYuv => writeln!(f, r#"Error: "No se pudo decodificar el frame YUV"#),
            Self::YuvFrameSettup => writeln!(f, r#"Error: "No se pudo configurar el frame YUV"#),
            Self::SetColorSpace => {
                writeln!(f, r#"Error: "No se pudo convertir el espacio de color"#)
            }
        }
    }
}
