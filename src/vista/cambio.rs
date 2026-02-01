#[derive(PartialEq, Clone)]
pub enum CambioVista {
    None,
    ToLogin,
    ToSignUp,
    ToLobby { username: String },
    ToCallStart { username: String },

    ToIncomingCall { caller: String },
    CalledUserBusy,
    CallAccepted { target_user: String },
    CallRejected { target_user: String },

    ToCall,
    ToCallEnd,
    CamEnable,
    CamDisable,

    SendFile, 
    SendFileQuit,
    SendFileConfirmed(String),
    SendFileRequested { from_user: String, filename: String },
    SendFileAccepted{ from_user: String, filename: String },
    SendFileRejected{ from_user: String, filename: String },
    SendFileNoticeClosed,

    TargetUserSendFileAccepted{ from_user: String, filename: String },
    TargetUserSendFileRejected{ from_user: String, filename: String },   

    MicEnable,
    MicDisable,
}
