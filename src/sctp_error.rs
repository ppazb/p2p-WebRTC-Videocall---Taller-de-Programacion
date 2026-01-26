use std::fmt;

use sctp_proto::AssociationError;


#[derive(Debug)]
pub enum SctpError {
    FileError, 
    FileCreationError, 
    HeaderSizeError, 
    HeaderParseError,
    HeaderIncomplete, 
    StreamOpenFailed,
    AssociationLost(AssociationError), 
    AssociationNotEstablished, 
    NoAvailableStreamId, 
    FilePathError, 
    FileMetaDataFail, 
    FilenameError, 
    FilePathCreationFailed
}

impl fmt::Display for SctpError{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SctpError::FileError => writeln!(f, "[SCTP] File Error falló apertura archivo"),
            SctpError::FileCreationError => writeln!(f, "[SCTP] Error en crear el archivo"),
            SctpError::HeaderParseError => writeln!(f, "[SCTP] Error de parseo de Header"),
            SctpError::HeaderSizeError => writeln!(f, "[SCTP] Tamaño del Header por debajo del valor mínimo"),
            SctpError::HeaderIncomplete => writeln!(f, "[SCTP] Header incompleto"),
            SctpError::AssociationLost(reason) => writeln!(f, "[SCTP] Association Event {:?}", reason),
            SctpError::AssociationNotEstablished => writeln!(f, "[SCTP] Association not established"),
            SctpError::NoAvailableStreamId => writeln!(f, "[SCTP] No hay Stream ID Libre para usar"),
            SctpError::StreamOpenFailed => writeln!(f, "[SCTP] Falló Open Stream"),
            SctpError::FilePathError => writeln!(f, "[SCTP] Error en obtener File Path"),
            SctpError::FileMetaDataFail => writeln!(f, "[SCTP] Error en obtener Metadata"),
            SctpError::FilenameError => writeln!(f, "[SCTP] Error en obtener Filename"),
            SctpError::FilePathCreationFailed => writeln!(f, "[SCTP] Creación de File Path fallido"),
        }
    }
}