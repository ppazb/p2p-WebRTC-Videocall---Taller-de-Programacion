use std::fmt;


#[derive(Debug)]
pub enum SctpError {
    ConnectionFailed,
    StreamNotFound,
    StreamWriteFailed,  
    SendFailed,
    ReceiveFailed,
    ReadFileError,
    FileError, 
    StreamReadFailed
}

impl fmt::Display for SctpError{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SctpError::ConnectionFailed => writeln!(f, "[SCTP] Connection Failed"),
            SctpError::StreamNotFound => writeln!(f, "[SCTP] Stream Not Found"),
            SctpError::StreamWriteFailed => writeln!(f, "[SCTP] Stream Write Failed"),
            SctpError::SendFailed => writeln!(f, "[SCTP] Send Failed"),
            SctpError::ReceiveFailed => writeln!(f, "[SCTP] Receive Failed"),
            SctpError::ReadFileError => writeln!(f, "[SCTP] Error reading file to send via SCTP"),
            SctpError::FileError => writeln!(f, "[SCTP] File error when trying to open file for SCTP sending"),
            SctpError::StreamReadFailed => writeln!(f, "[SCTP] Stream read failed"),
        }
    }
}