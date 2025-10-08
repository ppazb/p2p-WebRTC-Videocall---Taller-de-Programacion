// EJEMPLO
// v=0
// o=- 1234 1 IN IP4 127.0.0.1
// s=Demo
// t=0 0
// m=audio 49170 RTP/AVP 0
// a=rtpmap:0 PCMU/8000

use std::{fmt, fs::File, vec};
use std::io::Write;

use crate::sdp_errors::SdpError;

const ORIGIN_COMPONENTS_NUMBER: usize = 6;
const TIME_COMPONENTS_NUMBER: usize = 2;
const MEDIA_DESCRIPTION_COMPONENTS_NUMBER: usize = 4;


#[derive(Debug)]
pub enum SdpFileLineType {
    Version,
    Origin,
    SessionName,
    SessionTime,
    Media,
    Attribute,
    Unknown,
}

impl SdpFileLineType {
    pub fn parse_prefix(line: &str) -> Self {
        match line {
            "v" => SdpFileLineType::Version,
            "o" => SdpFileLineType::Origin,
            "s" => SdpFileLineType::SessionName,
            "t" => SdpFileLineType::SessionTime,
            "m" => SdpFileLineType::Media,
            "a" => SdpFileLineType::Attribute,
            _ => SdpFileLineType::Unknown,
        }
    }
}

#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct SessionDescription {
    version: usize,
    origin: Origin,
    session_name: String,
    session_time: Time,
    media: MediaDescription,
}

impl SessionDescription {
    pub fn new(
        version: usize,
        origin: Origin,
        session_name: String,
        session_time: Time,
        media: MediaDescription,
    ) -> Self {
        Self {
            version,
            origin,
            session_name,
            session_time,
            media,
        }
    }

    pub fn create_prov(local_addr: &str) -> Result<Self, SdpError> {

        if let Some((ip, port)) = local_addr.split_once(':') {
            let version = 0;
            let session_name = "demo";  
            let origin_struct = Origin::new(
            "sobrevivir_taller".to_string(), 
            1000,
            1, 
            "IN".to_string(), 
            "IP4".to_string(),
            ip.to_string()); 
            let time_struct = Time::new(0, 0);
            let media_description = MediaDescription::new(
                "video".to_string(), 
                port.to_string(), "UDP/TLS/RTP".to_string(), 
                97, 
                vec![]
            );

            Ok(Self::new(version, origin_struct, session_name.to_string(), time_struct, media_description))
        } else {
            Err(SdpError::InvalidadAdder)
        }
    }

    pub fn create(version: usize, origin: &str, session_name: &str, session_time: &str, media: &str, attributes: Vec<&str>) -> Result<Self, SdpError> {

        let origin_struct = Origin::parse(origin)?;
        let time_struct = Time::parse(session_time)?;
        let vec_attributes = Attribute::parse_vec_attributes(attributes)?; 
        let mut media_description = MediaDescription::parse(media)?;
        media_description.add_attr(vec_attributes);

        Ok(Self::new(version, origin_struct, session_name.to_string(), time_struct, media_description))
    }

    pub fn parse(text_sdp_protocol: &str) -> Result<SessionDescription, SdpError> {
        let mut version: Option<usize> = None;
        let mut origin: Option<Origin> = None;
        let mut session_name: Option<String> = None;
        let mut time: Option<Time> = None;
        let mut media: Option<MediaDescription> = None;
        let mut temp_vec_attrs = Vec::new();

        for line in text_sdp_protocol.lines() {
            println!("{}", line); 
            if let Some((attr, value)) = line.split_once('=') {
                match SdpFileLineType::parse_prefix(attr) {
                    SdpFileLineType::Version => version = Some(value.parse::<usize>()?),
                    SdpFileLineType::Origin => origin = Some(Origin::parse(value)?),
                    SdpFileLineType::SessionName => session_name = Some(value.to_string()),
                    SdpFileLineType::SessionTime => time = Some(Time::parse(value)?),
                    SdpFileLineType::Media => media = Some(MediaDescription::parse(value)?),
                    SdpFileLineType::Attribute => {
                        match Attribute::parse(value) {
                            Ok(a) => temp_vec_attrs.push(a),
                            Err(e) => return Err(e),
                        };
                    }
                    SdpFileLineType::Unknown => return Err(SdpError::ParseError),
                }
            }
        }

        if let Some(ref mut m) = media {
            m.add_attr(temp_vec_attrs);
        }

        Ok(Self::new(
            version.ok_or(SdpError::MissingField("v".to_string()))?,
            origin.ok_or(SdpError::MissingField("o".to_string()))?,
            session_name.ok_or(SdpError::MissingField("s".to_string()))?,
            time.ok_or(SdpError::MissingField("t".to_string()))?,
            media.ok_or(SdpError::MissingField("m".to_string()))?,
        ))
    }
}

impl fmt::Display for SessionDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "v={}", self.version)?;
        writeln!(f, "{}", self.origin)?;
        writeln!(f, "s={}", self.session_name)?;
        writeln!(f, "{}", self.session_time)?;
        writeln!(f, "{}", self.media)
    }
}

/// MEDIA DESCRIPTION
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct MediaDescription {
    media: String,
    port: String,
    transport_protocol: String,
    payload_type: usize,
    attributes: Vec<Attribute>,
}

impl MediaDescription {
    pub fn new(
        media: String,
        port: String,
        transport_protocol: String,
        payload_type: usize,
        attributes: Vec<Attribute>,
    ) -> Self {
        Self {
            media,
            port,
            transport_protocol,
            payload_type,
            attributes,
        }
    }

    pub fn add_attr(&mut self, new_attrs: Vec<Attribute>) {
        self.attributes.extend(new_attrs);
    }

    pub fn parse(str_mediadescrip: &str) -> Result<Self, SdpError> {
        let media_components: Vec<&str> = str_mediadescrip.split_whitespace().collect();

        if media_components.len() != MEDIA_DESCRIPTION_COMPONENTS_NUMBER {
            return Err(SdpError::InvalidMediaFormat);
        }

        Ok(Self::new(
            media_components[0].to_string(),
            media_components[1].to_string(),
            media_components[2].to_string(),
            media_components[3].parse::<usize>()?,
            Vec::new(),
        ))
    }
}

impl fmt::Display for MediaDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "m={} {} {} {}",
            self.media,
            self.port,
            self.transport_protocol,
            self.payload_type
        )?;
        for attr in &self.attributes {
            writeln!(f, "a={}", attr)?;
        }
        Ok(())
    }
}

/// ATTRIBUTE
#[derive(Debug, PartialEq, PartialOrd)]
pub enum Attribute {
    Property(String),
    Value(String, String),
}

impl Attribute {
    pub fn parse(str_attribute: &str) -> Result<Self, SdpError> {
        if let Some((key, value)) = str_attribute.split_once(':') {
            Ok(Attribute::Value(key.to_string(), value.to_string()))
        } else {
            Ok(Attribute::Property(str_attribute.to_string()))
        }
    }

    pub fn parse_vec_attributes(vec_attributes: Vec<&str>) -> Result<Vec<Attribute>, SdpError> {

        let mut vec_struct_attributes = Vec::new(); 

        for attr in vec_attributes {
            vec_struct_attributes.push(Self::parse(attr)?); 
        }

        Ok(vec_struct_attributes)
    }
}


impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Attribute::Property(k) => write!(f, "{}", k),
            Attribute::Value(k, v) => write!(f, "{}:{}", k, v),
        }
    }
}

/// STRUCT ORIGIN 
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct Origin {
    username: String,
    session_id: usize,
    session_version: usize,
    nettype: String,
    addrtype: String,
    unicast_address: String,
}

impl Origin {
    pub fn new(
        username: String,
        session_id: usize,
        session_version: usize,
        nettype: String,
        addrtype: String,
        unicast_address: String,
    ) -> Self {
        Self {
            username,
            session_id,
            session_version,
            nettype,
            addrtype,
            unicast_address,
        }
    }

    pub fn parse(str_origin: &str) -> Result<Self, SdpError> {
        let origin_components: Vec<&str> = str_origin.split_whitespace().collect();

        if origin_components.len() != ORIGIN_COMPONENTS_NUMBER {
            return Err(SdpError::InvalidOriginFormat);
        }

        Ok(Self::new(
            origin_components[0].to_string(),
            origin_components[1].parse::<usize>()?,
            origin_components[2].parse::<usize>()?,
            origin_components[3].to_string(),
            origin_components[4].to_string(),
            origin_components[5].to_string(),
        ))
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "o={} {} {} {} {} {}",
            self.username,
            self.session_id,
            self.session_version,
            self.nettype,
            self.addrtype,
            self.unicast_address
        )
    }
}

/// STRUCT TIME 
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct Time {
    start: usize,
    stop: usize,
}

impl Time {
    pub fn new(start: usize, stop: usize) -> Self {
        Self { start, stop }
    }

    pub fn parse(str_time: &str) -> Result<Self, SdpError> {
        let time_components: Vec<&str> = str_time.split_whitespace().collect();

        if time_components.len() != TIME_COMPONENTS_NUMBER {
            return Err(SdpError::InvalidTimeFormat);
        }

        Ok(Self::new(
            time_components[0].parse::<usize>()?,
            time_components[1].parse::<usize>()?,
        ))
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t={} {}", self.start, self.stop)
    }
}


// Provisoria para 2 clientes, después obtener los datos del .config 
pub fn generate_offer(session_descrip: &SessionDescription, file_path: &str) -> Result<File, SdpError> {

    let mut sdp_offer = match File::create(file_path) {
        Ok(f) => f, 
        Err(_) => return Err(SdpError::OfferFileCreationError)
    }; 
    
    match write!(sdp_offer, "{}", session_descrip) {
        Ok(_) => {},
        Err(_) => return Err(SdpError::OfferFileWriteError)
    };
    
    Ok(sdp_offer)
}

pub fn create_sdp_offer(local_adder: &str, file_path: &str) -> Result<SessionDescription, SdpError> {

    let session_description = SessionDescription::create_prov(local_adder)?; 

    let _offer = generate_offer(&session_description, file_path)?; 

    Ok(session_description)
}


#[test]
fn test_sdp_parsing() -> Result<(), SdpError> {
    let sdp_text = "\
            v=0
            o=- 1234 1 IN IP4 localhost
            s=TestSession
            t=0 0
            m=video 8081 UDP/TLS/RTP 97
            a=rtpmap:97 H264/90000
            ";

    let str_origin = String::from("- 1234 1 IN IP4 localhost");
    let sdp_correct_origin = Origin::parse(&str_origin)?;

    let str_time = String::from("0 0");
    let sdp_correct_time = Time::parse(&str_time)?;

    let str_session_name = String::from("TestSession");

    let str_media = String::from("video 8081 UDP/TLS/RTP 97");
    let mut sdp_correct_media = MediaDescription::parse(&str_media)?;

    let str_attr = "rtpmap:97 H264/90000";
    let sdp_correct_attribute = Attribute::parse(&str_attr)?;

    sdp_correct_media.add_attr(vec![sdp_correct_attribute]);

    let sdp_correct_session = SessionDescription::new(
        0,
        sdp_correct_origin,
        str_session_name,
        sdp_correct_time,
        sdp_correct_media,
    );

    // eprintln!("[DEBUG] SDP: {}", sdp_correct_session);
    if let Ok(new_session_sdp) = SessionDescription::parse(sdp_text) {
        assert_eq!(sdp_correct_session, new_session_sdp);
    }

    Ok(())
}
