use core::fmt;
#[derive(Debug)]
pub enum MessageErrors {
    CantCreate,
    MsgTooLarge,
}
///Implementacion para el manejo de error de mesanjes
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
