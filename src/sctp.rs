use std::{collections::VecDeque, fs::{self, File}, io::{self, BufRead, Read, Write}, net::{SocketAddr, UdpSocket}, path::Path, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{Receiver, TryRecvError}}, time::Instant};

use bytes::Bytes;
use openssl::ssl::SslStream;
use sctp_proto::{Association, AssociationEvent, AssociationHandle, ClientConfig, DatagramEvent, Endpoint, EndpointConfig, EndpointEvent, Event, Payload, PayloadProtocolIdentifier, ServerConfig, StreamEvent, Transmit};

use crate::{app, client, client_errors::ClientErrors, file_path::FilePathPacket, log_s, sctp_error::SctpError};

#[derive(Debug)]
struct IncomingFile {
    filename: Option<String>, 
    incoming_file: Option<File>,
    expected_size: Option<u64>,
    //header_buffer: Vec<u8>,
    written_size: u64, 
    header_completed: bool 
}

impl IncomingFile {
    pub fn new(filename: Option<String>, incoming_file: Option<File>, expected_size: Option<u64>, written_size: u64, header_completed: bool) -> Self {
        Self {
            filename: None,
            incoming_file: None,
            expected_size: None,
            //header_buffer: Vec::new(),
            written_size: 0,
            header_completed: false
        }
    }
}

#[derive(Debug)]
pub struct SctpStats {
    endpoint: Endpoint, 
    assoc: Option<Association>,
    assoc_handle: Option<AssociationHandle>,
    local: SocketAddr, 
    remote: SocketAddr, 
    pending_file: FilePathPacket,
    file: Option<IncomingFile>,

    hanshake_completed: Arc<AtomicBool>
}

fn flush_association_buffer(assoc: &mut Association, dtls_stream: &mut SslStream<MuxedUdpStream>, chunk: &Bytes) {
        let now = Instant::now(); 

        let mut stream = match assoc.stream(0) {
            Ok(s) => s, 
            Err(e) => {
                println!("[SCTP] STREAM ERROR en flush association buffer {:?}", e); 
                return; 
            }
        }; 

        stream.write_sctp(chunk, PayloadProtocolIdentifier::BinaryEmpty); 
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

impl SctpStats {
    pub fn new(local: SocketAddr, remote: SocketAddr, is_initiator: bool, pending_file: FilePathPacket) -> Self {

        let endpoint_config = EndpointConfig::new(); 
        if !is_initiator {
            let server_config = Some(Arc::new(ServerConfig::new())); 
            let endpoint = Endpoint::new(endpoint_config.into(), server_config); 
            println!("[SCTP] ENPOINT RECEPTOR de Archivo creado"); 

            Self {
                endpoint: endpoint, 
                assoc: None, 
                assoc_handle: None, 
                local: local,
                remote: remote,  
                pending_file: pending_file, 
                file: None, 
                hanshake_completed: Arc::new(AtomicBool::new(false))
            }
        } else {
            let endpoint = Endpoint::new(endpoint_config.into(), None); 
            println!("[SCTP] ENPOINT EMISOR de Archivo creado"); 

            Self {
                endpoint: endpoint, 
                assoc: None, 
                assoc_handle: None, 
                local: local,
                remote: remote,  
                pending_file: pending_file, 
                file: None,
                hanshake_completed: Arc::new(AtomicBool::new(false))
            }
        }
    }

    pub fn file_sender_handler(&mut self, dtls_stream: &mut SslStream<MuxedUdpStream>) {
        // CLIENT 
        let client_config = ClientConfig::new(); 
        if let Ok((assoc_handle, assoc)) = self.endpoint.connect(client_config, self.remote) {
            println!("[SCTP] CONEXIÓN EXITOSA: Se pudo conectar al Server Endpoint"); 
            self.assoc_handle = Some(assoc_handle); 
            self.assoc = Some(assoc); 
            self.transmit_pending(dtls_stream);

        } else {
            println!("[SCTP ERROR] CONEXIÓN FALLIDA: No se pudo conectar al Server Endpoint")
        }


        let mut buffer = [0u8; 10*1024];
        let handshake_succeded = Arc::clone(&self.hanshake_completed); 
        let header_sent = Arc::new(AtomicBool::new(false)); 

        loop {
            match dtls_stream.read(&mut buffer) {
                Ok(0) => {
                    println!("[SSLSTREAM] CONEXIÓN CERRADA"); 
                    break; 
                }
                Ok(n) => {
                    println!("[SSLSTREAM] Se leyeron: {:?}", &buffer[..n]); 
                    self.client_handle_event(&buffer[..n], &handshake_succeded);
                    self.transmit_pending(dtls_stream); 

                    if handshake_succeded.load(Ordering::SeqCst) && !header_sent.load(Ordering::SeqCst) {
                        self.send_file_header(&header_sent); 
                        self.transmit_pending(dtls_stream);
                        header_sent.store(true, Ordering::SeqCst);
                        self.send_file_content(dtls_stream);
                        self.transmit_pending(dtls_stream);
                    } 

                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // println!("[SSLSTREAM] {:?}", e); 
                    self.handle_timeout(dtls_stream);
                    continue
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // println!("[SSLSTREAM] {:?}", e); 
                    break; 
                },
                Err(e) => {
                    println!("[SSLSTREAM FATAL] Error Read SSLSTREAM {:?}", e); 
                    break; 
                }
            }
        }
        
    }

    fn client_handle_event(&mut self, sctp_pkt: &[u8], handshake_completed: &Arc<AtomicBool>) {
        let now = Instant::now(); 

        if let Some((_assoc_handle, datagram_ev)) = self.endpoint.handle(now, self.remote, None, None, sctp_pkt.to_vec().into()) {
            match datagram_ev {
                DatagramEvent::AssociationEvent(assoc_ev) => {
                    if let Some(association) = self.assoc.as_mut() {
                        association.handle_event(assoc_ev);

                            while let Some(event) = association.poll() {
                            match event {
                                Event::Connected => { 
                                    handshake_completed.store(true, Ordering::SeqCst);
                                    println!("[SCTP HANDSHAKE] SCTP HANDSHAKE COMPLETE")
                                }, 
                                Event::AssociationLost {reason} => { println!("[SCTP] ASSOCIATION LOST {:?}", reason)}
                                Event::Stream(stream_ev) => {
                                    match stream_ev {
                                        StreamEvent::Opened => println!("[SCPT] Stream Opened"), 
                                        StreamEvent::Available => println!("[SCPT] Stream Available"), 
                                        _  => {}
                                    }
                                }
                                _ => {}
                            }
                        }

                    }
                },
                _ => {}
            }
        }
    }

    fn send_file_header(&mut self, header_sent: &Arc<AtomicBool>) {
        if let Some(association) = self.assoc.as_mut() {

            let mut stream = match association.open_stream(0, PayloadProtocolIdentifier::Binary) {
                Ok(s) => s,
                Err(e) =>  {
                    println!("[SCTP] STREAM ERROR en Send Complete File {:?}", e); 
                    return; 
                }
            };

            // FILE PATH
            let file_path = match self.pending_file.get_path() {
                Some(fp) => fp, 
                None => {
                    println!("[SCTP] HEADER: No File Path"); 
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

            // [len nombre 2 bytes][nombre][len del file]
            let metadata = f.metadata().unwrap(); 
            let file_size = metadata.len(); 
            let filename = Path::new(file_path).file_name().unwrap().to_str().unwrap(); 
            let filename_bytes = filename.as_bytes(); 
            let filename_len = filename_bytes.len() as u16; 

            let mut header = Vec::new(); 
            header.extend_from_slice(&filename_len.to_be_bytes());
            header.extend_from_slice(filename_bytes);
            header.extend_from_slice(&file_size.to_be_bytes());

            stream.write_sctp(&Bytes::from(header), PayloadProtocolIdentifier::Binary); 
    
        }
    }

    fn send_file_content(&mut self, dtls_stream: &mut SslStream<MuxedUdpStream>) {

            let file_path = match self.pending_file.get_path() {
                Some(fp) => fp, 
                None => {
                    println!("[SCTP] HEADER: No File Path"); 
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
                        flush_association_buffer(association, dtls_stream, chunk);
                    }, 
                    Err(e) => {
                        println!("[SCTP] STREAM ERROR Envio Contenido: {:?}", e); 
                        break; 
                    }
                }
            }
            println!("[SCTP] File Enviado Completo"); 
            self.file = None; 
    }
    

    pub fn file_receiver_handler(&mut self, dtls_stream: &mut SslStream<MuxedUdpStream>) {
        // SERVER

        let mut buffer = [0u8; 10*1024];
        let handshake_succeded = Arc::clone(&self.hanshake_completed); 

        loop {
            // println!("[RECEIVER] Loop Receiver"); 
            match dtls_stream.read(&mut buffer) {
                Ok(0) => {
                    println!("[SSLSTREAM] CONEXIÓN CERRADA"); 
                    break; 
                }
                Ok(n) => {
                    println!("[SSLSTREAM] Se leyeron: {:?}", &buffer[..n]); 
                    self.server_handle_event(&buffer[..n], &handshake_succeded);
                    self.transmit_pending(dtls_stream); 
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // println!("[SSLSTREAM] {:?}", e); 
                    self.handle_timeout(dtls_stream);
                    continue
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // println!("[SSLSTREAM] {:?}", e); 
                    break; 
                },
                Err(e) => {
                    println!("[SSLSTREAM FATAL] Error Read SSLSTREAM {:?}", e); 
                    break; 
                }
            }
        }
    }

    // Función para handler los eventos del lado server (Receptor del Archivo). 
    // Recibe los packets en bytes leídos del SslStream
    fn server_handle_event(&mut self, sctp_pkt: &[u8], handshake_completed: &Arc<AtomicBool>) {
        println!("[HANDLE SCTP PACKET]"); 

        let now = Instant::now(); 
        let mut incoming_data: Vec<Vec<u8>> = Vec::new(); 
        if let Some((assoc_handle, datagram_ev)) = self.endpoint.handle(now, self.remote, None, None, sctp_pkt.to_vec().into()) {
            self.assoc_handle = Some(assoc_handle);
            match datagram_ev {
                DatagramEvent::AssociationEvent(assoc_ev) => {
                    println!("[SCTP] ASSOCIATION EVENT RECEIVED: {:?}", assoc_ev); 

                    if let Some(association) = self.assoc.as_mut() {

                        association.handle_event(assoc_ev);
                        if let Some(stream) =  association.accept_stream() {
                            if stream.stream_identifier() != 0 {
                                println!("[SCTP STREAM ERROR] No matchean STREAM ID")
                            }
                        }; 

                        while let Some(event) = association.poll() {
                            match event {
                                Event::Connected => { 
                                    handshake_completed.store(true, Ordering::SeqCst); 
                                    println!("[SCTP HANDSHAKE] SCTP HANDSHAKE COMPLETE")
                                }, 
                                Event::AssociationLost {reason} => { println!("[SCTP] ASSOCIATION LOST {:?}", reason)}
                                Event::Stream(stream_ev) => {
                                    match stream_ev {
                                        StreamEvent::Opened => println!("[SCPT] Stream Opened"), 
                                        StreamEvent::Available => println!("[SCPT] Stream Available"), 
                                        StreamEvent::Readable { id } => {
                                            if self.hanshake_completed.load(Ordering::SeqCst) {
                                               if let Ok(mut stream) = association.stream(id) {
                                                    while let Ok(Some(mut chunks)) = stream.read_sctp() {
                                                        println!("[SCTP] Se recibió un CHUNK"); 
                                                        let mut buffer = vec![0u8; 10*1024]; 
                                                        if let Ok(n) = chunks.read(&mut buffer) {
                                                            incoming_data.push(buffer[..n].to_vec()); 
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
                    println!("[SCTP NEW ASSOC] NEW ASSOCIATION RECEIVED: {:?}", new_assoc); 
                    self.assoc = Some(new_assoc); 
                }
            }

            for payload in incoming_data {
                println!("[SCTP] Procesando chunk de {} bytes...", payload.len());
                self.process_incoming_payload(&payload); 
            }
        };

    }

    fn process_incoming_payload(&mut self, payload: &[u8]) {
        if self.file.is_none() {
            println!("[SCTP SERVER] Recibiendo posible payload de Header ({} bytes)", payload.len()); 

            if payload.len() < 11 {
                eprintln!("[SERVER ERROR] Paquete muy pequeño para ser Header");
                return; 
            }

            let name_len = u16::from_be_bytes(payload[0..2].try_into().unwrap()) as usize;
            if payload.len() < 2 + name_len + 8 {
                eprintln!("[SERVER ERROR] Header incompleto");
                return; 
            }

            let name_bytes = &payload[2..(2 + name_len)];
            let filename = String::from_utf8_lossy(name_bytes).to_string();

            let size_start = 2 + name_len;
            let file_size = u64::from_be_bytes(payload[size_start..(size_start + 8)].try_into().unwrap());

            let folder = "downloads";
            let file_path = format!("{}/recibido_{}", folder, filename);
            if let Err(e) = fs::create_dir_all(folder) {
                eprintln!("[SERVER ERROR] No se pudo crear el directorio '{}': {:?}", folder, e);
                return;
            }
            let f = match File::create(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[SERVER ERROR] No se pudo crear archivo: {:?}", e);
                    return;
                }
            };

            self.file = Some(IncomingFile { filename: Some(filename), incoming_file: Some(f), expected_size: Some(file_size), written_size: 0, header_completed: true }); 

            let data_start = size_start + 8; 
            if payload.len() > data_start {
                let payload = &payload[data_start..]; 
                self.append_file(payload); 
            }
        } else {
            self.append_file(payload); 
        }
    }

    fn append_file(&mut self, data: &[u8]) {
        if let Some(incoming_f) = self.file.as_mut() {
            if let Some(file) = incoming_f.incoming_file.as_mut() {
                if let Err(e) = file.write_all(data) {
                    println!("[SCTP SERVER] Falló escritura en disco: {:?}", e);
                    return;
                }
            }
            
            incoming_f.written_size += data.len() as u64;
            println!("[SCTP SERVER] Progreso: {} / {} bytes", incoming_f.written_size, incoming_f.expected_size.unwrap_or(0)); 

            if incoming_f.written_size >= incoming_f.expected_size.unwrap_or(0) {
                println!("[SCTP SERVER] File Write Completed"); 
                self.file = None; 
            }
        }
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
            if let Some(timeout_chk) = assoc.poll_timeout() {
                if now >= timeout_chk {
                    assoc.handle_timeout(now);
                    self.transmit_pending(dtls_stream);
                }
            }
        }
    }

}




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
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "dtls datagram too big for read buffer"));
                }
                buf[..pkt.len()].copy_from_slice(&pkt);
                Ok(pkt.len())
            }
            Err(TryRecvError::Empty) => Err(io::Error::new(io::ErrorKind::WouldBlock, "no dtls data")),
            Err(TryRecvError::Disconnected) => Err(io::Error::new(io::ErrorKind::UnexpectedEof, "dtls channel closed")),
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



        // if let Some(association) = self.assoc.as_mut() {
        //     println!("[SCTP] CLIENT intentó OPEN STREAM"); 

        //     let mut stream = match association.open_stream(0, PayloadProtocolIdentifier::Binary) {
        //         Ok(s) => s, 
        //         Err(e) => {
        //             println!("[SCTP] Error OPEN STREAM {:?}", e); 
        //             return; 
        //         }
        //     }; 

        //     if stream.is_writable() {
        //         // Prueba
        //         let hello_msg = Bytes::from_static(b"Hello");
        //         stream.write_sctp(&hello_msg, PayloadProtocolIdentifier::Dcep); 
        //         self.transmit_pending(dtls_stream);

        //         println!("[SCTP] Se envío prueba Hello"); 
        //     } else {
        //         println!("[SCTP] Falló la creación del Stream"); 
        //     }
        // }

