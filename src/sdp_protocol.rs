// EJEMPLO
// v=0
// o=- 1234 1 IN IP4 127.0.0.1
// s=Demo
// t=0 0
// m=audio 49170 RTP/AVP 0
// a=rtpmap:0 PCMU/8000
// a=candidate: 196.168.0.1 80000

use crate::sdp_errors::SdpError;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::{fmt, fs::File, vec};

const ORIGIN_COMPONENTS_NUMBER: usize = 6;
const TIME_COMPONENTS_NUMBER: usize = 2;
const MEDIA_DESCRIPTION_COMPONENTS_NUMBER: usize = 4;

#[derive(Debug)]
enum SdpFileLineType {
    Version,
    Origin,
    SessionName,
    SessionTime,
    Media,
    Attribute,
    Unknown,
}

impl SdpFileLineType {
    fn parse_prefix(line: &str) -> Self {
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

    fn create_prov(local_addr: &str) -> Result<Self, SdpError> {
        if let Some((ip, port)) = local_addr.split_once(':') {
            let version = 0;
            let session_name = "demo";
            let origin_struct = Origin::new(
                "sobrevivir_taller".to_string(),
                1000,
                1,
                "IN".to_string(),
                "IP4".to_string(),
                ip.to_string(),
            );
            let time_struct = Time::new(0, 0);
            let candidate_str = format!("{} {}", ip, port);
            let media_description = MediaDescription::new(
                "video".to_string(),
                port.to_string(),
                "UDP/TLS/RTP".to_string(),
                97,
                vec![
                    Attribute::Value("rtpmap".to_string(), "97 H264/90000".to_string()),
                    Attribute::Value("candidate".to_string(), candidate_str),
                ],
            );

            Ok(Self::new(
                version,
                origin_struct,
                session_name.to_string(),
                time_struct,
                media_description,
            ))
        } else {
            Err(SdpError::InvalidAdder)
        }
    }

    // Solamente recibimos un MediaDescription (podría escalar a más y ser vector de MediaDescription)
    pub fn generate_compatible_session(
        &self,
        session_descrip_offer: &SessionDescription,
    ) -> Option<Self> {
        self.media
            .compare_media_descriptions(&session_descrip_offer.media)
            .map(|compatible_media| {
                Self::new(
                    self.version,
                    self.origin.clone(),
                    self.session_name.clone(),
                    self.session_time.clone(),
                    compatible_media,
                )
            })
    }

    pub fn parse(text_sdp_protocol: &str) -> Result<SessionDescription, SdpError> {
        let mut version: Option<usize> = None;
        let mut origin: Option<Origin> = None;
        let mut session_name: Option<String> = None;
        let mut time: Option<Time> = None;
        let mut media: Option<MediaDescription> = None;
        let mut temp_vec_attrs = Vec::new();

        for line in text_sdp_protocol.lines() {
            if let Some((attr, value)) = line.trim().split_once('=') {
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
    fn new(
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

    fn add_attr(&mut self, new_attrs: Vec<Attribute>) {
        self.attributes.extend(new_attrs);
    }

    fn get_codec(vec_attributes: &Vec<Attribute>) -> Result<&str, SdpError> {
        let codec = Attribute::get_rtpmap(vec_attributes)
            .and_then(|rtpmap| Attribute::parse_rtpmap(rtpmap))
            .map(|(codec, _)| codec)
            .ok_or(SdpError::CodecExtractionError)?;
        Ok(codec)
    }

    fn compare_media_descriptions(
        &self,
        another_media_descrip: &MediaDescription,
    ) -> Option<MediaDescription> {
        if self.media != another_media_descrip.media {
            return None;
        }

        let my_rtpmap = Attribute::get_rtpmap(&self.attributes)?;
        let (my_codec, my_clock) = Attribute::parse_rtpmap(my_rtpmap)?;
        let another_rtpmap = Attribute::get_rtpmap(&another_media_descrip.attributes)?;
        let (another_codec, another_clock) = Attribute::parse_rtpmap(another_rtpmap)?;

        if (my_codec == another_codec) && (my_clock == another_clock) {
            Some(MediaDescription::new(
                self.media.clone(),
                another_media_descrip.port.clone(),
                self.transport_protocol.clone(),
                self.payload_type,
                self.attributes.clone(),
            ))
        } else {
            None
        }
    }

    fn parse(str_mediadescrip: &str) -> Result<Self, SdpError> {
        // println!("[DEBUG] Media Parsing: {}", str_mediadescrip);
        let media_components: Vec<&str> = str_mediadescrip.split_whitespace().collect();

        if media_components.len() < MEDIA_DESCRIPTION_COMPONENTS_NUMBER {
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
            self.media, self.port, self.transport_protocol, self.payload_type
        )?;
        for attr in &self.attributes {
            writeln!(f, "a={}", attr)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
struct IceInformation {
    ufrag: String,
    pwd: String,
    candidates: Vec<String>,
}

/// ATTRIBUTE
#[derive(Debug, PartialEq, PartialOrd, Clone)]
enum Attribute {
    Property(String),
    Value(String, String),
}

impl Attribute {
    fn parse(str_attribute: &str) -> Result<Self, SdpError> {
        if let Some((key, value)) = str_attribute.split_once(':') {
            Ok(Attribute::Value(key.to_string(), value.to_string()))
        } else {
            Ok(Attribute::Property(str_attribute.to_string()))
        }
    }

    fn get_rtpmap(vec_attributes: &Vec<Attribute>) -> Option<&str> {
        for attr in vec_attributes {
            if let Attribute::Value(k, v) = attr
                && k.as_str() == "rtpmap"
            {
                return Some(v);
            }
        }

        None
    }

    fn parse_rtpmap(rtpmap: &str) -> Option<(&str, &str)> {
        if let Some((_, codec_clock)) = rtpmap.split_once(' ')
            && let Some((codec, clock)) = codec_clock.split_once('/')
        {
            return Some((codec, clock));
        }
        None
    }

    fn get_ice_candidate(vec_attributes: &Vec<Attribute>) -> Result<Vec<String>, SdpError> {
        let ice_candidates = Attribute::extract_ice_information(vec_attributes)?.candidates;
        Ok(ice_candidates)
    }

    fn extract_ice_information(
        vec_attributes: &Vec<Attribute>,
    ) -> Result<IceInformation, SdpError> {
        let mut ice_info = IceInformation {
            ufrag: String::new(),
            pwd: String::new(),
            candidates: Vec::new(),
        };

        for attr in vec_attributes {
            if let Attribute::Value(k, v) = attr {
                match k.as_str() {
                    "ice-ufrag" => ice_info.ufrag = String::from(v),
                    "ice-pwd" => ice_info.pwd = String::from(v),
                    "candidate" => {
                        let v = v.replace(" ", ":");
                        ice_info.candidates.push(v)
                    }
                    _ => {}
                }
            }
        }

        Ok(ice_info)
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
#[derive(Debug, Default, PartialEq, PartialOrd, Clone)]
pub struct Origin {
    username: String,
    session_id: usize,
    session_version: usize,
    nettype: String,
    addrtype: String,
    unicast_address: String,
}

impl Origin {
    fn new(
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

    fn parse(str_origin: &str) -> Result<Self, SdpError> {
        let origin_components: Vec<&str> = str_origin.split_whitespace().collect();

        if origin_components.len() != ORIGIN_COMPONENTS_NUMBER {
            println!("[DEBUG] Origin {:?}", origin_components);
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
#[derive(Debug, Default, PartialEq, PartialOrd, Clone)]
pub struct Time {
    start: usize,
    stop: usize,
}

impl Time {
    fn new(start: usize, stop: usize) -> Self {
        Self { start, stop }
    }

    fn parse(str_time: &str) -> Result<Self, SdpError> {
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
fn generate_offer(session_descrip: &SessionDescription, file_path: &str) -> Result<File, SdpError> {
    let mut sdp_offer = match File::create(file_path) {
        Ok(f) => f,
        Err(_) => return Err(SdpError::OfferFileCreationError),
    };

    match write!(sdp_offer, "offer\n{}", session_descrip) {
        Ok(_) => {}
        Err(_) => return Err(SdpError::OfferFileWriteError),
    };

    Ok(sdp_offer)
}

fn create_sdp_offer(local_adder: &str, file_path: &str) -> Result<SessionDescription, SdpError> {
    let session_description = SessionDescription::create_prov(local_adder)?;

    let _offer = generate_offer(&session_description, file_path)?;

    Ok(session_description)
}

// Provisoria para 2 clientes, después obtener los datos del .config
fn generate_answer(
    session_descrip_answer: &SessionDescription,
    file_path: &str,
) -> Result<File, SdpError> {
    let mut sdp_answer = match File::create(file_path) {
        Ok(f) => f,
        Err(_) => return Err(SdpError::AnswerFileCreationError),
    };

    match write!(sdp_answer, "answer\n{}", session_descrip_answer) {
        Ok(_) => {}
        Err(_) => return Err(SdpError::OfferFileWriteError),
    };

    Ok(sdp_answer)
}

fn create_sdp_answer(
    my_session_descripton: &SessionDescription,
    offer: &SessionDescription,
    file_path: &str,
) -> Result<SessionDescription, SdpError> {
    let session_descrip_answer = match my_session_descripton.generate_compatible_session(offer) {
        Some(answer) => answer,
        None => return Err(SdpError::IncompatibleSession),
    };

    match generate_answer(&session_descrip_answer, file_path) {
        Ok(_) => {}
        Err(e) => return Err(e),
    };

    Ok(session_descrip_answer)
}

fn try_connection(
    in_call: Arc<AtomicBool>,
    local_adder: &str,
    peer_adder: &str,
) -> Result<TcpStream, SdpError> {
    let listener = std::net::TcpListener::bind(local_adder).map_err(|_| SdpError::InvalidAdder)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| SdpError::Other(String::from("Non_blocking error")))?;

    let connected = Arc::new(AtomicBool::new(false));
    let connected_peer = Arc::clone(&connected);

    // Intento conectarme al peer 8001 desde otro thread
    let peer_address = peer_adder.to_string();
    let connection_peer = std::thread::spawn(move || {
        if connected_peer.load(std::sync::atomic::Ordering::SeqCst) {
            return None;
        };

        match std::net::TcpStream::connect(peer_address.as_str()) {
            Ok(stream) => {
                connected_peer.store(true, std::sync::atomic::Ordering::SeqCst);
                Some(stream)
            }
            Err(_) => None
        }
    });

    // Cualquiera de los dos que logre conectarse
    let mut connection_accepted: Option<TcpStream> = None;

    // Loop hasta que algún peer logre conectarse
    loop {
        // Pregunta si sigue el Thread de conexión al peer, si ya se conectó sale del loop
        // y usamos esa conexión
        if connected.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        if !in_call.load(std::sync::atomic::Ordering::SeqCst) {
            println!("Handshake cancelado");
            break;
        }

        if let Ok((stream, _)) = listener.accept() {
            connected.store(true, std::sync::atomic::Ordering::SeqCst);
            connection_accepted = Some(stream);
            break;
        };
    }

    let stream = match connection_accepted {
        Some(s) => {
            let _ = connection_peer.join();
            s
        }
        None => {
            if let Ok(Some(stream)) = connection_peer.join() {
                stream
            } else {
                return Err(SdpError::InvalidAdder);
            }
        }
    };

    Ok(stream)
}

// Realiza el Handshake SDP entre dos peers, intercambiando a través de TCPSocket que simula el pasaje por Signalling Server
// devuelve 3 Vectores, uno con las direcciones ICE del peer local, otro con las direcciones ICE del peer remoto y con los codecs
pub fn sdp_handshake(
    in_call: Arc<AtomicBool>,
    local_adder: &str,
    peer_adder: &str,
) -> Result<(Vec<String>, Vec<String>, Vec<String>), SdpError> {
    println!(
        "\n\n[DEBUG] Iniciando SDP Handshake entre {} y {}\n",
        local_adder, peer_adder
    );

    // Ej: local 196.168.0.109:8000
    //     peer 196.168.0.109:8001

    // Abro el listener TCP y escucho en mi local 8000
    let local_adder_signalling = local_adder.to_string();
    let peer_adder_singnalling = peer_adder.to_string();
    let mut stream = try_connection(in_call, &local_adder_signalling, &peer_adder_singnalling)?;

    let offer_my_session = create_sdp_offer(local_adder, "src/my_offer.sdp")?;
    let mut my_ice_candidates =
        Attribute::extract_ice_information(&offer_my_session.media.attributes)?.candidates;

    // Para separar quién envía el Offer y quién recibe el Offer y envía el Answer
    if local_adder < peer_adder {
        // Soy quién envía mi offer y recibo el Answer de mi peer

        println!("[DEBUG] Lado Enviar Offer: {:?}\n", offer_my_session);

        // Levanto mi Offer del archivo
        let my_offer = fs::read_to_string("src/my_offer.sdp")
            .map_err(|_| SdpError::Other(String::from("Read my_offer error")))?;
        let client_answer_session_description;

        // Envío mi Offer por el TcpStream de signalling
        stream
            .write_all(my_offer.as_bytes())
            .map_err(|_| SdpError::OfferFileWriteError)?;
        stream
            .flush()
            .map_err(|_| SdpError::Other(String::from("[LADO ENVIO OFFER] Flush error")))?;
        let _ = stream.shutdown(std::net::Shutdown::Write);

        // Espero para leer el Answer del peer
        let mut reader = BufReader::new(&mut stream);
        let mut signal_str = String::new();
        let _ = reader.read_to_string(&mut signal_str);

        let _ = stream.shutdown(std::net::Shutdown::Read);

        // SessionDescription del Answer recibido
        match signal_str.strip_prefix("answer\n") {
            Some(answer_client) => {
                client_answer_session_description = SessionDescription::parse(answer_client)?;
                println!(
                    "[DEBUG] LADO OFFER Answer Recibido: {:?}\n",
                    client_answer_session_description
                );
            }
            None => {
                return Err(SdpError::IncompatibleSession);
            }
        };

        // Parseo y obtengo el Codec compartido, mis ICE Candidates y los del peer
        let codec =
            MediaDescription::get_codec(&client_answer_session_description.media.attributes)?;

        let mut ice_candidates_peer =
            Attribute::get_ice_candidate(&client_answer_session_description.media.attributes)?;
        ice_candidates_peer.push(peer_adder.to_string());
        my_ice_candidates.push(local_adder.to_string());

        Ok((
            my_ice_candidates,
            ice_candidates_peer,
            vec![codec.to_string()],
        ))
    } else {
        // Soy quien recibo el Offer y creo el Answer

        println!("[DEBUG] Lado Enviar Answer: {:?}\n", offer_my_session);

        let mut reader = BufReader::new(&mut stream);
        let mut signal_str = String::new();
        let _ = reader.read_to_string(&mut signal_str);

        let _ = stream.shutdown(std::net::Shutdown::Read);

        // SessionDescription del Offer recibido
        let sdp_offer: SessionDescription = SessionDescription::default();
        // SessionDescription de mi Answer a generar
        let answer_peer_session;

        // Parseo del Offer recibido y creo mi Answer
        match signal_str.strip_prefix("offer\n") {
            Some(offer_sent) => {
                let sdp_offer = SessionDescription::parse(offer_sent)?;
                println!("[DEBUG] LADO ANSWER Offer Recibido: {:?}\n", sdp_offer);

                answer_peer_session =
                    create_sdp_answer(&offer_my_session, &sdp_offer, "src/my_answer.sdp")?;
                println!(
                    "[DEBUG] LADO ANSWER Answer Creado: {:?}\n",
                    answer_peer_session
                );

                let answer = fs::read_to_string("src/my_answer.sdp")
                    .map_err(|_| SdpError::Other(String::from("Read answer_peer error")))?;

                stream
                    .write_all(answer.as_bytes())
                    .map_err(|_| SdpError::IncompatibleSession)?;
                stream
                    .flush()
                    .map_err(|_| SdpError::Other(String::from("Flush error")))?;
                let _ = stream.shutdown(std::net::Shutdown::Write);
            }
            None => return Err(SdpError::IncompatibleSession),
        }

        // Parse y obtengo el Codec compartido, mis ICE Candidates y los del peer
        // en este caso los ICE candidates del peer vienen del Offer recibido
        let codec = MediaDescription::get_codec(&answer_peer_session.media.attributes)?;
        let mut ice_candidates_peer = Attribute::get_ice_candidate(&sdp_offer.media.attributes)?;
        ice_candidates_peer.push(peer_adder.to_string());
        my_ice_candidates.push(local_adder.to_string());

        Ok((
            my_ice_candidates,
            ice_candidates_peer,
            vec![codec.to_string()],
        ))
    }
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
    let sdp_correct_attribute = Attribute::parse(str_attr)?;

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
