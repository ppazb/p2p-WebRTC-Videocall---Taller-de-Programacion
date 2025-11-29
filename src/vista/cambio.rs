#[derive(PartialEq, Clone)]
pub enum CambioVista {
    None,
    ToLogin,
    ToSignUp,
    ToLobby { username: String },
    ToCallStart { username: String },

    ToIncomingCall { caller: String },
    CallAccepted { target_user: String },
    CallRejected { target_user: String },

    ToCall,
    ToCallEnd,
    CamEnable,
    CamDisable,
}
