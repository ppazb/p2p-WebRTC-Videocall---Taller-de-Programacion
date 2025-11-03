use crate::mensaje_errors::MessageErrors;

#[derive(Debug, Clone, Copy)]
pub enum MsgType {
    Texto = 0,
    Audio = 1,
    Video = 2,
    Signalling = 3,
}

pub enum Content {
    Text(String),
    Binary(Vec<u8>),
    Signal(Signal),
}

#[derive(Debug, Clone, Copy)]
pub enum Signal {
    EndCall = 0,
}
///Estructura para abstraer y encapsular que se envia por UDP.
/// La idea es poder tener una interfaz flexible que nos permita luego del lado del receptor 
/// saber cual era la intencion del mensaje.
pub struct Message {
    pub typemsg: MsgType,
    pub dimension: (i32, i32),
    pub content: Content,
}

impl Message {
    ///Distintos tipos de constructures para las distintas necesidades
    pub fn new_msg_text(text: &str) -> Self {
        Self {
            typemsg: MsgType::Texto,
            content: Content::Text(text.to_string()),
            dimension: (0, 0),
        }
    }

    pub fn new_msg_signalling(signal: Signal) -> Self {
        Self {
            typemsg: MsgType::Signalling,
            content: Content::Signal(signal),
            dimension: (0, 0),
        }
    }

    pub fn new_msg_binary(video: Vec<u8>, _width: i32, _height: i32) -> Self {
        Self {
            typemsg: MsgType::Video,
            content: Content::Binary(video),
            dimension: (_width, _height),
        }
    }

    ///Convertimos el mensaje a bytes sea cual sea el mensaje para poder enviarlo por sockets
    /// lo unico que varia es su fragmentacion segun el tipo de mensaje a enviar
    /// Se serializa con un headder el cual contiene la iformacion relevante para su deserializacion
    pub fn serialize(&self) -> Result<Vec<u8>, MessageErrors> {
        let mut serialize = Vec::new();
        serialize.push(self.typemsg as u8);
        serialize.extend_from_slice(&self.dimension.0.to_be_bytes());
        serialize.extend_from_slice(&self.dimension.1.to_be_bytes());
        match &self.content {
            Content::Text(text) => {
                let text_bytes = text.as_bytes();
                let len_text = (text_bytes.len() as u32).to_be_bytes();
                serialize.extend_from_slice(&len_text);
                serialize.extend_from_slice(text_bytes);
            }
            Content::Binary(data) => {
                let len_data = (data.len() as u32).to_be_bytes();
                serialize.extend_from_slice(&len_data);
                serialize.extend_from_slice(data);
            }
            Content::Signal(signal) => {
                // Serializo la variante enum como un único byte
                let signal_byte = *signal as u8;
                serialize.extend_from_slice(&(1u32.to_be_bytes()));
                serialize.push(signal_byte);
            }
        }
        Ok(serialize)
    }

    ///Cuando recibimos un mensaje en bytes lo recuperamos para leer su contenido
    /// utilizando el header del mensaje sabemos como reconstruirlo
    /// siguiendo una estructura del estilo | header| content  |
    /// el primer byte contiene el tipo de mensaje
    /// luego la segunda  y tercera parte contiene largo y ancho del frame (solo utilizado en video)
    /// 4 bytes para cada uno. Finalmente tenemos otros 4 bytes para el largo del mensaje
    /// por lo cual una vez reconstruido todas las partes del headder podemos saber que todo lo que sigue desde
    /// 13 bytes +  len es el contenido del mensaje perse. En base a su tipo sabemos como reconstruirlo
    /// en caso de funcionar correctamente devulve un nuevo mensaje con los valores del anterior
    /// caso contrario None
    pub fn deserializable(data: &[u8]) -> Option<Self> {
        if data.len() < 13 {
            return None;
        }

        let msgtype = match data[0] {
            0 => MsgType::Texto,
            1 => MsgType::Audio,
            2 => MsgType::Video,
            3 => MsgType::Signalling,
            _ => return None,
        };

        let w = i32::from_be_bytes([data[1], data[2], data[3], data[4]]);
        let h = i32::from_be_bytes([data[5], data[6], data[7], data[8]]);
        let len = u32::from_be_bytes([data[9], data[10], data[11], data[12]]) as usize;
        if data.len() < 13 + len {
            return None;
        }

        let content_bytes = &data[13..13 + len];
        let content = match msgtype {
            MsgType::Texto => Content::Text(String::from_utf8_lossy(content_bytes).to_string()),
            MsgType::Signalling => {
                // Identifico el byte de la señal recibida
                if len == 1 {
                    let signal = match content_bytes[0] {
                        0 => Signal::EndCall,
                        _ => return None,
                    };
                    Content::Signal(signal)
                } else {
                    Content::Text(String::from_utf8_lossy(content_bytes).to_string())
                }
            }
            _ => Content::Binary(content_bytes.to_vec()),
        };

        Some(Self {
            content,
            typemsg: msgtype,
            dimension: (w, h),
        })
    }
}
