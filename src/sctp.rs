use std::{fs::{self, File}, io::{self, Read, Write}, net::{SocketAddr, UdpSocket}, path::Path, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{Receiver, TryRecvError}}, time::Instant};

use bytes::Bytes;
use eframe::egui::ahash::{HashSet, HashSetExt};
use std::collections::HashMap; 
use openssl::ssl::SslStream;
use sctp_proto::{Association, AssociationHandle, ClientConfig, DatagramEvent, Endpoint, EndpointConfig, Event, Payload, PayloadProtocolIdentifier, ServerConfig, StreamEvent, Transmit};

use crate::{file_path::FilePathPacket, sctp_error::SctpError};

const MAX_STREAM_ID: usize = 32768; 

/// ALMACENAMIENTO ARCHIVO RECIBIDO
/// Estructura para almacenar los datos del archivo recibido. Se almacena el nombre del archivo, el tamaño esperado, la cantidad
/// de bytes que ya fueron escritos y el handle del archivo
#[derive(Debug)]
struct IncomingFile {
    // filename: Option<String>, 
    incoming_file: Option<File>,
    expected_size: Option<u64>,
    written_size: u64
}

impl IncomingFile {
    pub fn new(_filename: Option<String>, incoming_file: Option<File>, expected_size: Option<u64>) -> Self {
        Self {
            // filename: filename,
            incoming_file: incoming_file,
            expected_size: expected_size,
            //header_buffer: Vec::new(),
            written_size: 0
        }
    }

    // Método que verifica si la escritura del archivo está finalizado
    // compara la cantidad de bytes escritos con la cantidad de bytes esperados
    // si es igual o mayor significa que ya se completó 
    pub fn is_complete(&self) -> bool {
        if let Some(ex_size) = self.expected_size {
            self.written_size >= ex_size
        } else {
            eprintln!("[INCOMING FILE] Expected Size is None"); 
            false
        }
    }
}

/// ALMACENAMIENTO ESTADO DE SCTP 
/// Almacena los datos para la transmisión por Data Channel bajo el protocolo SCTP
/// Mantiene un HashMap asociando cada Stream ID con su respectivo archivo
#[derive(Debug)]
pub struct SctpStats {
    endpoint: Endpoint, 
    assoc: Option<Association>,
    assoc_handle: Option<AssociationHandle>, 
    remote: SocketAddr, 
    pending_file: Option<FilePathPacket>,

    incoming_files: HashMap<u16, IncomingFile>, 
    occupied_stream_id: HashSet<u16>,

    handshake_completed: Arc<AtomicBool>, 
    is_initiator: bool
}

// Función principal de la lógica de envío y recepción de archivos. 
// 1. Crea un SctpStats una vez que inicia la llamada el Peer obtiene el rol de Client si 
//    es quien inicia la llamada y el Peer que recibe la llamada toma el rol de Server
// 2. El lado client crea un ClientConfig y se intenta conectar al lado Server 
// 3. Mientras se mantiene en llamada, leemos del SslStream (Socket UDP encriptado bajo DTLS) 
//    Con el repartidor agregado en Receiver thread de Client, me aseguro que los paquetes que llegan 
//    son paquetes DTLS. 
// 4. Handleo los paqeutes con la función handle_network_data. Despuéś de cada acción hago flush al buffer 
//    para que envié los paquetes SCTP por el SslStream 
// 5. Si recibo un FilePathPakcet (con Path del archivo) significa que el Peer eligió enviar un archivo 
//    Si el SCTP Handshake ya se completó y hay archivo pendiente para enviar, se envía primero el Header
//    para crear el Strcut IncomingFile. 
// 6. Se envía por último el contenido del archivo 
pub fn start_sctp_thread(remote_adder: SocketAddr, is_initiator: &Arc<AtomicBool>, in_call: &Arc<AtomicBool>, rx_file: Receiver<FilePathPacket>, mut dtls_stream: SslStream<MuxedUdpStream>) -> Result<(), SctpError> {

    let mut sctpstats = SctpStats::new(remote_adder, is_initiator, None);

    if is_initiator.load(Ordering::SeqCst) {
        let client_config = ClientConfig::new(); 
        if let Ok((assoc_handle, assoc)) = sctpstats.endpoint.connect(client_config, sctpstats.remote) {
            sctpstats.assoc = Some(assoc);
            sctpstats.assoc_handle = Some(assoc_handle); 
            sctpstats.transmit_pending(&mut dtls_stream);
        }
    }

    let mut buffer = [0u8; 10*1024];

    while in_call.load(Ordering::SeqCst) {
        if let Ok(file_pkt) = rx_file.try_recv() {
            println!("[SCTP] Nueva orden de envío detectada: {:?}", &file_pkt.get_path());
            sctpstats.pending_file = Some(file_pkt); 
        }
    

        match dtls_stream.read(&mut buffer) {

                Ok(0) => {
                    println!("[SSLSTREAM] CONEXIÓN CERRADA"); 
                    break; 
                },
                Ok(n) => {
                    println!("[DTLS] Nuevos datos leídos: {:?}", &buffer[..n]);
                    if let Err(e) = sctpstats.handle_network_data(&buffer[..n], &mut dtls_stream) {
                        return Err(e); 
                    };
                    sctpstats.transmit_pending(&mut dtls_stream);
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No hay datos de red, manejamos timeouts internos (retransmisiones/heartbeats)
                    sctpstats.handle_timeout(&mut dtls_stream);
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    println!("[DTLS] DTLS UNEXPECTED EOF {:?}", e); 
                    break
                },
                Err(e) => {
                    println!("[SSLSTREAM FATAL] Error Read SSLSTREAM {:?}", e); 
                    break; 
                }
        }
        
        // println!("[DEBUG] Handshake: {}, Pending: {}", 
        //     sctpstats.handshake_completed.load(Ordering::SeqCst), 
        //     sctpstats.pending_file.is_some()
        // );

        // Si se completó SCTP Handshake y hay archivo pendiente 
        // Enviar primero el Header y después el contenido del archivo. 
        if sctpstats.handshake_completed.load(Ordering::SeqCst) && sctpstats.pending_file.is_some() {
            println!("[SCTP] Arranco con el envío de Archivo"); 
            let selected_stream_id = match sctpstats.send_file_header() {
                Ok(stream) =>  stream, 
                Err(e) => return Err(e)
            };
            sctpstats.transmit_pending(&mut dtls_stream);
            sctpstats.send_file_content(&mut dtls_stream, selected_stream_id);
            sctpstats.transmit_pending(&mut dtls_stream);
            sctpstats.pending_file = None; 
            println!("[SCTP] Sistema listo para el próximo archivo.");
        }

        sctpstats.transmit_pending(&mut dtls_stream);
    }
    Ok(())
}

impl SctpStats {
    pub fn new(remote: SocketAddr, is_initiator: &Arc<AtomicBool>, pending_file: Option<FilePathPacket>) -> Self {

        let endpoint_config = EndpointConfig::new(); 
        if !is_initiator.load(Ordering::SeqCst) {
            let server_config = Some(Arc::new(ServerConfig::new())); 
            let endpoint = Endpoint::new(endpoint_config.into(), server_config); 
            println!("[SCTP] ENPOINT RECEPTOR de Archivo creado"); 

            Self {
                endpoint: endpoint, 
                assoc: None, 
                assoc_handle: None,
                remote: remote,  
                pending_file: pending_file, 
                occupied_stream_id: HashSet::new(), 
                incoming_files: HashMap::new(),
                handshake_completed: Arc::new(AtomicBool::new(false)), 
                is_initiator: false
            }
        } else {
            let endpoint = Endpoint::new(endpoint_config.into(), None); 
            println!("[SCTP] ENPOINT EMISOR de Archivo creado"); 

            Self {
                endpoint: endpoint, 
                assoc: None, 
                assoc_handle: None, 
                remote: remote,  
                pending_file: pending_file, 
                occupied_stream_id: HashSet::new(), 
                incoming_files: HashMap::new(),
                handshake_completed: Arc::new(AtomicBool::new(false)),
                is_initiator: true
            }
        }
    }

    /// Función principal que handlea los datagrams entrantes. Si son paquetes del protocolo se resuelven usando handle_event
    /// Si es una una associación se guarda y si es paquete de datos de archivo se procesa como datos de archivo
    pub fn handle_network_data(&mut self, data: &[u8], dtls_stream: &mut SslStream<MuxedUdpStream>) -> Result<(), SctpError> {
        println!("[HANDLE SCTP PACKET]");
        let now = Instant::now();
        let mut incoming_payloads = Vec::new();

        if let Some((_, datagram_ev)) = self.endpoint.handle(now, self.remote, None, None, data.to_vec().into()) {
            match datagram_ev {
                DatagramEvent::AssociationEvent(assoc_ev) => {
                    println!("[SCTP] ASSOCIATION EVENT RECEIVED: {:?}", assoc_ev); 
                    if let Some(assoc) = self.assoc.as_mut() {
                        assoc.handle_event(assoc_ev);

                        if let Some(_) =  assoc.accept_stream() {
                                println!("[SCTP STREAM] A Stream has been accepted")
                        }; 
                        
                        while let Some(event) = assoc.poll() {
                            match event {
                                Event::Connected => {
                                    println!("[SCTP HANDSHAKE] SCTP HANDSHAKE COMPLETE"); 
                                    self.handshake_completed.store(true, Ordering::SeqCst);
                                }
                                Event::AssociationLost {reason} => { 
                                    println!("[SCTP] ASSOCIATION LOST {:?}", reason); 
                                    return Err(SctpError::AssociationLost(reason)); 
                                }
                                Event::Stream(stream_ev) => {
                                    match stream_ev {
                                        StreamEvent::Opened => println!("[SCPT] Stream Opened"), 
                                        StreamEvent::Available => println!("[SCPT] Stream Available"), 
                                        StreamEvent::Readable { id } => {
                                            if self.handshake_completed.load(Ordering::SeqCst) {
                                               if let Ok(mut stream) = assoc.stream(id) {
                                                    while let Ok(Some(chunks)) = stream.read_sctp() {
                                                        println!("[SCTP] Se recibió un CHUNK"); 
                                                        let mut buffer = vec![0u8; 10*1024]; 
                                                        if let Ok(n) = chunks.read(&mut buffer) {
                                                            incoming_payloads.push((id, buffer[..n].to_vec())); 
                                                        }
                                                    }
                                               }
                                            }
                                        }
                                        _  => {}
                                    }
                                }
                                Event::DatagramReceived => {}
                            }
                        }
                    }
                }
                DatagramEvent::NewAssociation(new_assoc) => {
                    self.assoc = Some(new_assoc);
                }
            }
        }

        self.transmit_pending(dtls_stream);
        for (id, payload) in incoming_payloads {
            if let Err(e) = self.process_incoming_payload_by_stream(id, &payload) {
                return Err(e); 
            };
        }

        Ok(())
    }

    // Función que devuelve un Stream ID disponible, el lado Client usa IDs pares y el lado Server
    // usa IDs impares 
    fn get_available_stream_id(occupied_stream_id: &HashSet<u16>, is_initiator: bool) -> Option<u16> {

        if occupied_stream_id.len() > MAX_STREAM_ID {
            return None; 
        }
        
        let mut possible_id = if is_initiator { 0u16 } else { 1u16 }; 

        while occupied_stream_id.contains(&possible_id) {
            possible_id = possible_id.wrapping_add(2u16); 
        }

        Some(possible_id)
    }

    // Función que se encarga de enviar el Header del archivo con el tamaño del nombre del archivo, el nombre del archivo 
    // y el tamaño del archivo a ser enviado
    fn send_file_header(&mut self) -> Result<u16, SctpError> {
        if let Some(association) = self.assoc.as_mut() {

            let stream_id = match Self::get_available_stream_id(&self.occupied_stream_id, self.is_initiator) {
                Some(id) => id, 
                None => {
                    eprintln!("[SCTP] No hay Stream ID libre"); 
                    return Err(SctpError::NoAvailableStreamId); 
                } 
            }; 

            self.occupied_stream_id.insert(stream_id); 
            let mut stream = match association.stream(stream_id) {
                Ok(s) => s,
                Err(_) => {
                    match association.open_stream(stream_id, PayloadProtocolIdentifier::Binary) {
                        Ok(s) => s,
                        Err(e) => {
                            println!("[SCTP ERROR] No se pudo abrir el stream {}: {:?}", stream_id, e);
                            return Err(SctpError::StreamOpenFailed); 
                        }
                    }
                }
            };

            let file_path = match self.pending_file.as_ref().and_then(|pkt| pkt.get_path()) {
                Some(fp) => fp,
                None => {
                    println!("[SCTP] ERROR: Intentó enviar pero no hay path en pending_file");
                    return Err(SctpError::FilePathError);
                }
            };

            let f = match File::open(file_path) {
                Ok(f) => f, 
                Err(e) =>  {
                    println!("[SCTP] OPEN FILE ERROR: File Open Failed {:?}", e); 
                    return Err(SctpError::FileError); 
                }
            }; 

            // [len nombre 2 bytes][nombre][len del file]
            let metadata = f.metadata().map_err(|_| SctpError::FileMetaDataFail)?; 
            let file_size = metadata.len(); 

            let filename_os = Path::new(file_path).file_name().ok_or(SctpError::FileCreationError)?; 

            let filename = filename_os.to_str().ok_or(SctpError::FilenameError)?; 
            let filename_bytes = filename.as_bytes(); 
            let filename_len = filename_bytes.len() as u16; 

            let mut header = Vec::new(); 
            header.extend_from_slice(&filename_len.to_be_bytes());
            header.extend_from_slice(filename_bytes);
            header.extend_from_slice(&file_size.to_be_bytes());

            let _ = stream.write_sctp(&Bytes::from(header), PayloadProtocolIdentifier::Binary); 
            return Ok(stream_id); 
        }

        return Err(SctpError::AssociationNotEstablished); 
    }

    // Función que se necarga de mandar el contenido del archivo. sctp-proto se encarga de Chunkear el archivo 
    fn send_file_content(&mut self, dtls_stream: &mut SslStream<MuxedUdpStream>, selected_stream_id: u16) {

        let file_path = match self.pending_file.as_ref().and_then(|pkt| pkt.get_path()) {
            Some(fp) => fp,
            None => {
                println!("[SCTP] ERROR: Intentó enviar pero no hay path en pending_file");
                return;
            }
        };

            let f = match File::open(file_path) {
                Ok(f) => f, 
                Err(e) =>  {
                    println!("[SCTP] OPEN FILE ERROR: File Open Failed {:?}", e); 
                    return; 
                }
            }; 

            // ENVIO CONTENIDO
            let mut buffer = [0u8; 10*1024]; 
            let mut reader = std::io::BufReader::new(f); 

            let association = match self.assoc.as_mut() {
                Some(a) => a, 
                None => {
                    println!("[SCTP] Send File Content NO ASSOC"); 
                    return; 
                }
            }; 

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = &Bytes::copy_from_slice(&buffer[..n]); 
                        flush_association_buffer(association, dtls_stream, chunk, selected_stream_id);
                    }, 
                    Err(e) => {
                        println!("[SCTP] STREAM ERROR Envio Contenido: {:?}", e); 
                        break; 
                    }
                }
            }
            println!("[SCTP] File Enviado Completo"); 
            self.occupied_stream_id.remove(&selected_stream_id); 
            println!("[SCTP] Stream ID {} liberado", selected_stream_id); 
            self.pending_file = None; 
    }
    
    // Procesa el payload entrante, usamos un stream para cada archivo 
    fn process_incoming_payload_by_stream(&mut self, stream_id: u16, payload: &[u8]) -> Result<(), SctpError> {

        if !self.incoming_files.contains_key(&stream_id) {
            // Nuevo archivo

            if payload.len() < 11 {
                eprintln!("[SERVER ERROR] Paquete muy pequeño para ser Header");
                return Err(SctpError::HeaderSizeError); 
            }

            let name_len_bytes: [u8; 2] = payload[0..2].try_into().map_err(|_| SctpError::HeaderParseError)?; 
            let name_len = u16::from_be_bytes(name_len_bytes) as usize; 

            // let name_len = u16::from_be_bytes(payload[0..2].try_into().unwrap()) as usize;
            if payload.len() < 2 + name_len + 8 {
                eprintln!("[SERVER ERROR] Header incompleto");
                return Err(SctpError::HeaderIncomplete); 
            }

            let name_bytes = &payload[2..(2 + name_len)];
            let filename = String::from_utf8_lossy(name_bytes).to_string();

            let size_start = 2 + name_len;
            let file_size_bytes  = payload[size_start..(size_start + 8)].try_into().map_err(|_| SctpError::HeaderParseError)?; 
            let file_size = u64::from_be_bytes(file_size_bytes); 
            // let file_size = u64::from_be_bytes(payload[size_start..(size_start + 8)].try_into().unwrap());

            let folder = "downloads";
            let file_path = format!("{}/recibido_{}", folder, filename);
            if let Err(e) = fs::create_dir_all(folder) {
                eprintln!("[SERVER ERROR] No se pudo crear el directorio '{}': {:?}", folder, e);
                return Err(SctpError::FileCreationError);
            }
            let f = match File::create(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[SERVER ERROR] No se pudo crear archivo: {:?}", e);
                    return Err(SctpError::FileCreationError);
                }
            };

            let new_file = IncomingFile::new(Some(filename), Some(f), Some(file_size)); 
            self.incoming_files.insert(stream_id, new_file); 
        } else {

            if let Some(incoming) = self.incoming_files.get_mut(&stream_id) {
                if let Some(file) = incoming.incoming_file.as_mut() {
                    let _ = file.write_all(payload); 
                }
                incoming.written_size += payload.len() as u64; 

                if incoming.is_complete() {
                    println!("[SCTP] Archivo en Stream {} finalizado.", stream_id);
                    self.incoming_files.remove(&stream_id);
                }
            }
        }

        Ok(())
    }

    // Enviar el Payload del Transmit por el SslStream
    fn send_to_stream(transmit: Transmit, dtls_stream: &mut SslStream<MuxedUdpStream>) {

        if let Payload::RawEncode(contents) = transmit.payload {
            for content in contents {
                if let Err(e) = dtls_stream.write_all(&content) {
                    println!("[DTLS] DTLS STREAM write en SCTP error: {:?}", e); 
                }; 
            }   
        }
    }

    fn transmit_pending(&mut self, dtls_stream: &mut SslStream<MuxedUdpStream>) {
        let now = Instant::now(); 

        while let Some(transmit) = self.endpoint.poll_transmit() {
            Self::send_to_stream(transmit, dtls_stream);
        }

        if let Some(association) = self.assoc.as_mut() {
            while let Some(transmit) = association.poll_transmit(now) {
                Self::send_to_stream(transmit, dtls_stream);
            }
        } 
    }

    fn handle_timeout(&mut self, dtls_stream: &mut SslStream<MuxedUdpStream>) {
        let now = Instant::now(); 

        if let Some(assoc) = self.assoc.as_mut() {
            if let Some(timeout_chk) = assoc.poll_timeout() 
                && now >= timeout_chk {
                    assoc.handle_timeout(now);
                    self.transmit_pending(dtls_stream);
                }
            }
        
    }

}

fn flush_association_buffer(assoc: &mut Association, dtls_stream: &mut SslStream<MuxedUdpStream>, chunk: &Bytes, stream_id: u16) {
    let now = Instant::now(); 

    let mut stream = match assoc.stream(stream_id) {
        Ok(s) => s, 
        Err(e) => {
            println!("[SCTP] STREAM ERROR en flush association buffer {:?}", e); 
            return; 
        }
    }; 

    let _ = stream.write_sctp(chunk, PayloadProtocolIdentifier::BinaryEmpty); 
    while let Some(transmit) = assoc.poll_transmit(now) {
        if let Payload::RawEncode(contents) = transmit.payload {
            for content in contents {
                if let Err(e) = dtls_stream.write_all(&content) {
                    println!("[DTLS] DTLS STREAM write en SCTP error: {:?}", e); 
                }; 
            }   
        }
    }
} 


/// Struct MuxedUpStream que toma la entrada (Read) del Channel de Receiver thread que filtra los paquetes DTLS 
/// y hace write en UdpSocket 
#[derive(Debug)]
pub struct MuxedUdpStream {
    udp: UdpSocket,
    dtls_in: Receiver<Vec<u8>>,
}

impl MuxedUdpStream {
    pub fn new(udp: UdpSocket, dtls_in: Receiver<Vec<u8>>) -> Self {
        let _ = udp.set_nonblocking(true);
        Self { udp, dtls_in }
    }
}

impl Read for MuxedUdpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.dtls_in.try_recv() {
            Ok(pkt) => {
                if pkt.len() > buf.len() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "DTLS Datagram supera el tamaño del buffer"));
                }
                buf[..pkt.len()].copy_from_slice(&pkt);
                Ok(pkt.len())
            }
            Err(TryRecvError::Empty) => Err(io::Error::new(io::ErrorKind::WouldBlock, "No se recibió paquetes DTLS")),
            Err(TryRecvError::Disconnected) => Err(io::Error::new(io::ErrorKind::UnexpectedEof, "DTLS Channel cerrado")),
        }
    }
}

impl Write for MuxedUdpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.udp.send(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

