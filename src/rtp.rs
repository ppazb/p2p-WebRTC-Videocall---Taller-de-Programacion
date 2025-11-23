use chrono::Utc;
#[derive(Debug, Clone)]
pub struct RtpPackage {
    pub version: u8,
    pub padding: bool,
    pub extension: bool,
    pub csrc_count: u8,
    pub maker: bool,
    pub payload_type: u8,
    pub secuence_number: u16,
    pub time_stamp: u32,
    pub ssrc: u32,
    pub payload: Vec<u8>,
}

impl RtpPackage {
    ///contructor de un paquete RTP minimo, genera un indice y saca un timestamp al momento de creacion
    /// para futura identificacion, agrega el conetido util al payload
    pub fn new(index: u16, payload: Vec<u8>) -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            maker: false,
            payload_type: 96,
            secuence_number: index,
            time_stamp: Utc::now().timestamp_millis() as u32,
            ssrc: 0,
            payload,
        }
    }
    /// convierte el paquete RTP a bytes asegurando un orden de header para su futura reconstruccion
    /// todo el headder debe entrar en 12 bytes, por lo tanto se aprobecha el tamaño de los atributos
    /// y se realizan las acomodaciones necesarias para que tenga sentido, es decir valores bool
    /// son "acomodados" en un solo byte, para economizar espacio
    /// devuelve el vector de bytes
    pub fn to_bytes(self) -> Vec<u8> {
        let mut output = Vec::with_capacity(12 + self.payload.len());
        let byte0 = (self.version << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | (self.csrc_count & 0x0F);
        let byte1 = ((self.maker as u8) << 7) | (self.payload_type & 0x7F);
        output.push(byte0);
        output.push(byte1);
        output.extend_from_slice(&self.secuence_number.to_be_bytes());
        output.extend_from_slice(&self.time_stamp.to_be_bytes());
        output.extend_from_slice(&self.ssrc.to_be_bytes());
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
        let version: u8 = (data[0] >> 6) & 0x03;
        let padding = (data[0] & 0x20) != 0;
        let extension = (data[0] & 0x10) != 0;
        let csrs_count = data[0] & 0x0F;
        let maker = (data[1] & 0x80) != 0;
        let payload_type = data[1] & 0x7F;
        let index: u16 = u16::from_be_bytes([data[2], data[3]]);
        let time_stamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let payload = if data.len() > 12 {
            data[12..].to_vec()
        } else {
            Vec::new()
        };
        Some(Self {
            version,
            padding,
            extension,
            csrc_count: csrs_count,
            maker,
            payload_type,
            secuence_number: index,
            time_stamp,
            ssrc,
            payload,
        })
    }
}
