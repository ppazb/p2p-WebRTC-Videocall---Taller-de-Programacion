use core::fmt;
#[derive(Debug)]
pub enum LoginErrors {
    UserNotFound,
    IncorrectPassword,
    EmptyFields,
    FileNotFound,
    FileIncompatible,
    OneUserMultipleLogins,
    UsernameAlreadyExists,
    InvalidFieldsOnSignUp,
}

impl fmt::Display for LoginErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginErrors::UserNotFound => write!(f, "Usuario no encontrado."),
            LoginErrors::IncorrectPassword => write!(f, "Contraseña incorrecta."),
            LoginErrors::EmptyFields => write!(f, "Los campos no pueden estar vacíos."),
            LoginErrors::FileNotFound => write!(f, "Archivo de usuarios no encontrado."),
            LoginErrors::FileIncompatible => write!(
                f,
                "El archivo debe ser Json y cumplir con el formato esperado."
            ),
            LoginErrors::OneUserMultipleLogins => {
                write!(f, "Un usuario no puede iniciar sesión múltiples veces.")
            }
            LoginErrors::UsernameAlreadyExists => write!(f, "El nombre de usuario ya existe."),
            LoginErrors::InvalidFieldsOnSignUp => write!(f, "Credenciales inválidas, intente nuevamente."),
        }
    }
}
