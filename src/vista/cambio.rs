#[derive(PartialEq)]
pub enum CambioVista {
    None,
    ToLogin,
    ToLobby,
    ToCallStart,
    ToCall,
    ToCallEnd,
    CamEnable,
    CamDisable,
}
