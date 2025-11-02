use chrono::Utc;
#[derive(Debug, Clone)]
pub struct RtpPackage {
    pub index: u32,
    pub time_stamp: u64,
    pub payload: Vec<u8>,
}

impl RtpPackage {
    ///contructor de un paquete RTP minimo, genera un indice y saca un timestamp al momento de creacion
    /// para futura identificacion, agrega el conetido util al payload
    pub fn new(index: u32, payload: Vec<u8>) -> Self {
        Self {
            index,
            time_stamp: Utc::now().timestamp_millis() as u64,
            payload,
        }
    }
    /// convierte el paquete RTP a bytes asegurando un orden de header para su futura reconstruccion
    /// devuelve el vector de bytes
    pub fn to_bytes(self) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(&self.index.to_be_bytes());
        output.extend_from_slice(&self.time_stamp.to_be_bytes());
        output.extend_from_slice(&self.payload);
        output
    }
    /// recive una tira de bytes y trata de reconstruir el paquete RTP
    /// siguiendo la construccion definida la estructura primero el indice, luego el timestamp
    /// finalmente el payload devuelve un paquete RTP en caso de funcionar sino None
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        let index: u32 = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let time_stamp = u64::from_be_bytes([
            data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
        ]);
        let payload = if data.len() > 12 {
            data[12..].to_vec()
        } else {
            Vec::new()
        };
        Some(Self {
            index,
            time_stamp,
            payload,
        })
    }
}
