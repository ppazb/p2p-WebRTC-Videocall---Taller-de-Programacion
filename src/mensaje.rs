use crate::room_rtc_errors::MessageErrors;

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
}

pub struct Message {
    pub typemsg: MsgType,
    pub content: Content,
}

impl Message {
    pub fn new_msg_text(text: &str) -> Self {
        Self {
            typemsg: MsgType::Texto,
            content: Content::Text(text.to_string()),
        }
    }

    pub fn new_msg_signalling(text: &str) -> Self {
        Self {
            typemsg: MsgType::Signalling,
            content: Content::Text(text.to_string()),
        }
    }

    pub fn new_msg_binary() {}
    //Convertimos el mensaje a bytes sea cual sea el mensaje
    pub fn serialize(&self) -> Result<Vec<u8>, MessageErrors> {
        let mut serialize = Vec::new();
        serialize.push(self.typemsg as u8);
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
        }
        Ok(serialize)
    }
    //Cuando recibimos un mensaje en bytes lo recuperamos para leer su contenido
    pub fn deserializable(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let msgtype = match data[0] {
            0 => MsgType::Texto,
            1 => MsgType::Video,
            2 => MsgType::Audio,
            3 => MsgType::Signalling,
            _ => return None,
        };
        let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
        if data.len() < 5 + len {
            return None;
        }
        let content_bytes = &data[5..5 + len];
        let content = match msgtype {
            MsgType::Texto => Content::Text(String::from_utf8_lossy(content_bytes).to_string()),
            MsgType::Signalling => {
                Content::Text(String::from_utf8_lossy(content_bytes).to_string())
            }
            _ => Content::Binary(content_bytes.to_vec()),
        };
        Some(Self {
            content,
            typemsg: msgtype,
        })
    }
}
