use crate::camera::Camera;
use crate::client_errors::ClientErrors;
use crate::encoding::{DecoderRtc, EncoderRtc};
use crate::mensaje::Message;
use crate::mensaje::{Content, Signal};
use crate::rtcp::{Bye, ReceiverReport, ReportBlock, RtcpPacket, SenderReport};
use crate::rtp::RtpPackage;
use crate::sdp_protocol::*;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::core::{Mat, MatTraitConst};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::vec;
use std::{net::UdpSocket, thread};
extern crate dimpl;
#[derive(Debug, Default)]
pub struct RtcStats {
    // Sender stats
    pub packets_sent: u32,
    pub bytes_sent: u32,

    // Receiver stats
    pub packets_received: u32,
    pub bytes_received: u32,
    pub max_seq_received: u16,
    pub base_seq: u16,
    pub cycles: u16,
    pub jitter: f64,
    pub last_pkt_timestamp: u32,
    pub last_arrival_time: Option<Instant>,
    pub initialized: bool,
}

struct JitterBuffer {
    buffer: BTreeMap<u32, Vec<u8>>,
    min_depth: usize, // min de frames antes de empezar a reproducir
    max_depth: usize, // evitar latencia infinita
}

impl JitterBuffer {
    fn new() -> Self {
        Self {
            buffer: BTreeMap::new(),
            min_depth: 4,
            max_depth: 30,
        }
    }

    fn push(&mut self, timestamp: u32, data: Vec<u8>) {
        // evito desborde
        if self.buffer.len() >= self.max_depth {
            // si esta arriba de la cant max priorizamos lo mas nuevo
            if let Some(&k) = self.buffer.keys().next() {
                self.buffer.remove(&k);
            }
        }
        self.buffer.insert(timestamp, data);
    }

    fn pop(&mut self) -> Option<Vec<u8>> {
        if self.buffer.len() >= self.min_depth
            && let Some(&key) = self.buffer.keys().next()
        {
            return self.buffer.remove(&key);
        }
        None
    }
}

#[derive(Debug)]
pub struct Client {
    socket_send: Option<UdpSocket>,
    socket_recev: Option<UdpSocket>,
    tx_remoto: SyncSender<Mat>,
    in_call: Arc<AtomicBool>,
    is_active_comunicator: Arc<AtomicBool>,
    camera: Arc<Mutex<Camera>>,
    stats: Arc<Mutex<RtcStats>>,
}

impl Client {
    ///Constructor de cliente, requiere de un socket para generar un receptor y un emisor
    /// tambien recive channel para comunicarse con la UI.
    pub fn new(
        tx_remoto: SyncSender<Mat>,
        in_call: Arc<AtomicBool>,
        camera: Arc<Mutex<Camera>>,
    ) -> Result<Self, ClientErrors> {
        Ok(Self {
            socket_recev: None,
            socket_send: None,
            tx_remoto,
            in_call,
            is_active_comunicator: Arc::new(AtomicBool::new(false)),
            camera,
            stats: Arc::new(Mutex::new(RtcStats::default())),
        })
    }

    pub fn add_peer(&mut self, socket: UdpSocket) -> Result<(), ClientErrors> {
        socket.set_read_timeout(Some(Duration::from_millis(50)))?;
        self.socket_recev = Some(socket.try_clone().map_err(ClientErrors::CantClone)?);
        self.socket_send = Some(socket);
        Ok(())
    }

    pub fn set_rol(&self, is_client: bool) {
        self.is_active_comunicator
            .store(is_client, Ordering::SeqCst);
    }
    ///Se encarga de matchear y aplicaar el paseo de SDP
    pub fn handle_signalling(&self, text: String) {
        let peer_session_description = match SessionDescription::parse(&text) {
            Ok(peer_sd) => peer_sd,
            Err(e) => return eprintln!("{}", e),
        };
        println!("[Session Description] {}", peer_session_description);
    }

    ///Inicia la comunicacion entre 2 clientes genera 2 hilos de ejecucion uno como receptor y otro como emsior
    /// Dentro de cada hilo recive los componentes para poder comunicarse con la UI.
    /// Ademas cada hilo levanta los recursos necesarios: camara, decoder, encoder que necesite.
    /// Finalmente llama a metodo privados para encargar de toda la logica y manejo interno de la comunicacion.
    pub fn initialize_call(
        &self,
    ) -> Result<Vec<JoinHandle<Result<(), ClientErrors>>>, ClientErrors> {
        let socket_recev = self
            .socket_recev
            .as_ref()
            .ok_or(ClientErrors::SocketNotInitialized)?
            .try_clone()
            .map_err(ClientErrors::CantClone)?;
        let socket_send = self
            .socket_send
            .as_ref()
            .ok_or(ClientErrors::SocketNotInitialized)?
            .try_clone()
            .map_err(ClientErrors::CantClone)?;
        let socket_rtcp_out = socket_send.try_clone().map_err(ClientErrors::CantClone)?;

        let tx_remoto = self.tx_remoto.clone();
        let in_call = Arc::clone(&self.in_call);
        let stats_recv = Arc::clone(&self.stats);

        if self.perform_dtls_handshake().is_err() {
            println!("NO FUNCO DTLS");

            return Err(ClientErrors::SocketNotInitialized);
        }

        let receiver_thread = thread::spawn(move || -> Result<(), ClientErrors> {
            let res = Self::reciver(
                &socket_recev,
                &socket_rtcp_out,
                &tx_remoto,
                &in_call,
                &stats_recv,
            );
            if let Err(ref e) = res {
                eprintln!("[CRITICAL] Receiver thread died: {:?}", e);
            }
            res
        });

        let in_call = Arc::clone(&self.in_call);
        let camera = Arc::clone(&self.camera);
        let stats_sender = Arc::clone(&self.stats);

        let sender_thread = thread::spawn(move || -> Result<(), ClientErrors> {
            let res = Self::sender(&socket_send, &camera, &in_call, &stats_sender);
            if let Err(ref e) = res {
                eprintln!("[CRITICAL] Sender thread died: {:?}", e);
            }
            res
        });

        Ok(vec![receiver_thread, sender_thread])
    }

    ///Levanta la camara, y se queda constantemente enviando mensajes por medio de la funcion
    /// hasta encontrarse con la señal de terminar,luego de esto enviar el mensaje de "fin" de llamada
    /// realiza este envio varias veces para asegurar que llegue a destino por estar utilizando UDP
    /// Al detectar la señal de fin de llamada le comunica esta decision a la aplicacion para un cierre correcto
    fn sender(
        socket_send: &UdpSocket,
        camera: &Arc<Mutex<Camera>>,
        in_call: &Arc<AtomicBool>,
        stats: &Arc<Mutex<RtcStats>>,
    ) -> Result<(), ClientErrors> {
        let mut encoder = EncoderRtc::new_encoding()?;
        let mut last_sender_report_time = Instant::now();
        let sender_report_interval = Duration::from_secs(5);

        loop {
            // Corte de llamada y notificación remota
            if !in_call.load(Ordering::SeqCst) {
                let bye = Bye::new(1234, Some("Call ended".into()));
                let packet_rtcp = RtcpPacket::Bye(bye);
                let _ = socket_send.send(&packet_rtcp.to_bytes());

                let msg = Message::new_msg_signalling(Signal::EndCall);
                for _ in 0..3 {
                    if let Ok(bytes) = msg.serialize() {
                        let _ = socket_send.send(&bytes);
                    }
                }
                return Ok(());
            }

            Self::send_message(socket_send, camera, &mut encoder, stats)?;

            if last_sender_report_time.elapsed() >= sender_report_interval {
                let (packets, octets) = {
                    let s = stats.lock().map_err(|_e| ClientErrors::StatsCouldtGets)?;
                    (s.packets_sent, s.bytes_sent)
                };
                let ntp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros() as u64;
                let rtp_ts = (ntp / 1000) as u32;

                let sender_report = SenderReport::new(1234, ntp, rtp_ts, packets, octets);
                let packet = RtcpPacket::SenderReport(sender_report);
                if let Err(e) = socket_send.send(&packet.to_bytes()) {
                    eprintln!("[RTCP] Error sending SR: {}", e);
                }
                last_sender_report_time = Instant::now();
            }
        }
    }
    ///Crea un decoder, y una tabla de hash y luego se queda escuchando constantemente por el socket
    /// detecta el tipo de mensaje que se recive y delega la responsabilidad a la funcion determinada
    /// pasando los recursos necesarios para permitir el funcionamiento correcto
    /// la estructura esta pensada para ser escalable y agregar funcionalidad sea un proceso simple
    /// En caso de detectar el fin de la llamada comunica esta decision a la aplicacion para un correcto cierre.
    fn reciver(
        socket_recev: &UdpSocket,
        socket_rtcp_out: &UdpSocket,
        tx_remoto: &SyncSender<Mat>,
        in_call: &Arc<AtomicBool>,
        stats: &Arc<Mutex<RtcStats>>,
    ) -> Result<(), ClientErrors> {
        let mut decoder = DecoderRtc::new_decoder()?;
        // Hash map de los fragmentos rtp
        let mut chunks_hash: HashMap<u32, Vec<(u16, Vec<u8>)>> = HashMap::new();
        // variables para setear el ancho y alto del futuro frame
        let mut width = 0;
        let mut height = 0;
        let mut jitter_buffer = JitterBuffer::new();

        let mut buff = vec![0u8; 65000];
        let mut last_reciever_report = Instant::now();
        let reciever_report_interval = Duration::from_secs(5);

        loop {
            // Controlo el flag in_call para salir del thread
            if !in_call.load(Ordering::SeqCst) {
                return Ok(());
            }

            match socket_recev.recv(&mut buff) {
                Ok(n) => {
                    let data = &buff[..n];
                    if n < 2 {
                        continue;
                    }

                    let package_type: u8 = data[1];
                    let mut processed_as_rtcp = false;

                    if (200..=204).contains(&package_type)
                        && let Some(package) = RtcpPacket::from_bytes(data)
                    {
                        processed_as_rtcp = true;
                        if let RtcpPacket::Bye(_) = package {
                            println!("[RTCP] Recibido BYE. Cerrando llamada.");
                            in_call.store(false, Ordering::SeqCst);
                            return Ok(());
                        }
                    }
                    if !processed_as_rtcp && let Some(msg) = Message::deserializable(data) {
                        match msg.content {
                            Content::Signal(signal) => Self::signal_handle(signal, in_call),
                            Content::Text(_text) => continue,
                            Content::Binary(media) => {
                                if let Some(rtp_package) = RtpPackage::from_bytes(&media) {
                                    Self::update_rtp_stats(stats, &rtp_package, n);
                                }

                                // Intentamos reensamblar el frame
                                if let Some((ts, full_frame)) =
                                    Self::reassemble_frame(media, &mut chunks_hash)
                                {
                                    // Si tenemos frame completo mando al jitter y seteo el alto y ancho del frame
                                    jitter_buffer.push(ts, full_frame);
                                    width = msg.dimension.0;
                                    height = msg.dimension.1;
                                }
                            }
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::TimedOut
                        || e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(ClientErrors::CantConect(e)),
            }

            // sacamos el frame del arbol
            if let Some(encoded_frame) = jitter_buffer.pop()
                && let Ok(frame) = decoder.decode(&encoded_frame, width, height)
            {
                let _ = tx_remoto.try_send(frame);
            }

            if last_reciever_report.elapsed() >= reciever_report_interval {
                Self::send_receiver_report(socket_rtcp_out, stats)?;
                last_reciever_report = Instant::now();
            }
        }
    }

    /// Devuelve Some((timestamp, frame_data)) si completamoos un frame
    /// Modifica el chunks_hash internamente
    fn reassemble_frame(
        video_chunk: Vec<u8>,
        chunks_hash: &mut HashMap<u32, Vec<(u16, Vec<u8>)>>,
    ) -> Option<(u32, Vec<u8>)> {
        let chunk_size = 1200;

        if let Some(rtp_package) = RtpPackage::from_bytes(&video_chunk) {
            let time_stamp = rtp_package.time_stamp;

            // Verificamos si completamos el frame con este chunk
            let is_complete = {
                let entry = chunks_hash.entry(time_stamp).or_default();
                entry.push((rtp_package.secuence_number, rtp_package.payload));
                entry.last().map(|(_, p)| p.len()).unwrap_or(0) < chunk_size
            };

            if is_complete && let Some(mut entry) = chunks_hash.remove(&time_stamp) {
                entry.sort_by_key(|(i, _)| *i);
                let mut full: Vec<u8> = Vec::new();
                for (_, p) in entry.drain(..) {
                    full.extend(p);
                }
                return Some((time_stamp, full));
            }
        }
        None
    }
    /// Se encarga de recibir el frame de la camara y encodearlo.
    /// Luego envia el frame tanto a la UI como por mensaje de video a los clientes.
    /// Antes de enviar el mensaje se asegura de crear un paquete RTC con el mensaje serializado
    /// para esto indexea cada parte de un mismo mensaje en un bucle para su posterior remsamblaje
    fn send_message(
        socket_send: &UdpSocket,
        camera: &Arc<Mutex<Camera>>,
        encoder: &mut EncoderRtc,
        stats: &Arc<Mutex<RtcStats>>,
    ) -> Result<(), ClientErrors> {
        let chunk_size: usize = 1200;

        // Tomo el lock de la cámara, y cargo el frame más reciente
        let (frame, camara_width, camara_height) = {
            let cam = camera.lock().map_err(|_| {
                ClientErrors::VideoProcessingError(VideoProcessingErrors::AccessCamara)
            })?;

            if let Some(f) = cam.get_last_frame() {
                let width = f.cols();
                let height = f.rows();
                (f, width, height)
            } else {
                // No hay frame disponible
                return Ok(());
            }
        };

        // Encodeo el frame cargado, y lo serializo para enviar los datos por el socket
        let encoded_frame = encoder
            .encode(&frame)
            .map_err(ClientErrors::EncodingError)?;
        if encoded_frame.is_empty() {
            return Ok(());
        };

        let mut index_chunk: u16 = 0;
        let mut bytes_sent_batch = 0;
        let mut package_sent_batch = 0;

        for chunk in encoded_frame.chunks(chunk_size) {
            let rtp_package = RtpPackage::new(index_chunk, chunk.to_vec());
            let rtp_bytes = rtp_package.to_bytes();
            let video_to_send = Message::new_msg_binary(rtp_bytes, camara_width, camara_height);
            let bytes_to_send = video_to_send
                .serialize()
                .map_err(|_e| ClientErrors::SerializeBits)?;

            if let Err(e) = socket_send.send(&bytes_to_send) {
                eprintln!("Send failed: {}", e);
            } else {
                bytes_sent_batch += bytes_to_send.len() as u32;
                package_sent_batch += 1;
            }
            index_chunk = index_chunk.wrapping_add(1);
        }

        if let Ok(mut client_stats) = stats.lock().map_err(|_e| ClientErrors::StatsCouldtGets) {
            client_stats.packets_sent += package_sent_batch;
            client_stats.bytes_sent += bytes_sent_batch;
        }
        Ok(())
    }
    ///Mantiene la consistencia del arc RtcStats,principalemente toda la firma temporal de la estructura
    /// tamaño y datos del header del paquete RTP eviado/recibido
    fn update_rtp_stats(stats: &Arc<Mutex<RtcStats>>, rtp: &RtpPackage, packet_len: usize) {
        if let Ok(mut update_stats) = stats.lock().map_err(|_e| ClientErrors::StatsCouldtGets) {
            let now = Instant::now();
            if !update_stats.initialized {
                update_stats.base_seq = rtp.secuence_number;
                update_stats.max_seq_received = rtp.secuence_number;
                update_stats.last_pkt_timestamp = rtp.time_stamp;
                update_stats.last_arrival_time = Some(now);
                update_stats.initialized = true;
            }
            update_stats.packets_received += 1;
            update_stats.bytes_received += packet_len as u32;

            let seq = rtp.secuence_number;
            if seq < update_stats.max_seq_received
                && (update_stats.max_seq_received as i32 - seq as i32) > 30000
            {
                update_stats.cycles += 1;
            }
            if seq > update_stats.max_seq_received {
                update_stats.max_seq_received = seq;
            }

            if let Some(last_arrival) = update_stats.last_arrival_time {
                let dif_arrival = now.duration_since(last_arrival).as_millis() as f64;
                let dif_timestamp =
                    (rtp.time_stamp as f64 - update_stats.last_pkt_timestamp as f64).abs();
                let diference: f64 = (dif_arrival - dif_timestamp).abs();
                update_stats.jitter += (diference - update_stats.jitter) / 16.0;
            }
            update_stats.last_pkt_timestamp = rtp.time_stamp;
            update_stats.last_arrival_time = Some(now);
        }
    }

    fn send_receiver_report(
        socket: &UdpSocket,
        stats: &Arc<Mutex<RtcStats>>,
    ) -> Result<(), ClientErrors> {
        let rb = {
            let client_stats = stats.lock().map_err(|_e| ClientErrors::StatsCouldtGets)?;
            let extended_max =
                (client_stats.cycles as u32) << 16 | client_stats.max_seq_received as u32;
            let expected = if client_stats.initialized {
                extended_max - client_stats.base_seq as u32 + 1
            } else {
                0
            };
            let lost = expected.saturating_sub(client_stats.packets_received);

            ReportBlock {
                ssrc: 0,
                fraction_lost: 0,
                cumulative_lost: lost,
                ext_seq_num: extended_max,
                jitter: client_stats.jitter as u32,
                last_sr_timestamp: 0,
                delay_since_last_sr: 0,
            }
        };

        let recieve_report = ReceiverReport::new(1234, vec![rb]);
        let package = RtcpPacket::ReceiverReport(recieve_report);
        let _ = socket.send(&package.to_bytes());
        Ok(())
    }

    fn signal_handle(signal: Signal, in_call: &Arc<AtomicBool>) {
        match signal {
            Signal::EndCall => {
                in_call.store(false, Ordering::SeqCst);
            }
        }
    }
    ///Se encarga de realizar el hadnshake DTLS con el create dimpl, su gestion es cuasi
    /// automatica puesto que el crate se encarga tambien de la creacion del certificado
    /// y matchea si el hadnshake fue exitoso.
    /// El unico factor importante es el seteo de "cliente activo" para saber quien actuara de cliente
    /// y quien de servidor. Si el handshake falla la comunicacion deberia cortarse puesto que no es segura
    pub fn perform_dtls_handshake(&self) -> Result<bool, ClientErrors> {
        let certificate = dimpl::certificate::generate_self_signed_certificate()?;
        let config = Arc::new(dimpl::Config::default());
        let mut dtls = dimpl::Dtls::new(config, certificate.certificate, certificate.private_key);

        let mut buf_in = vec![0u8; 2048];
        let mut buf_out = vec![0u8; 2048];
        dtls.set_active(self.is_active_comunicator.load(Ordering::SeqCst));
        if let Err(e) = dtls.handle_timeout(Instant::now()) {
            eprintln!("DTLS error: {:?}", e);
        }

        let handshake_start = Instant::now();
        let handshake_timeout = Duration::from_secs(10);
        loop {
            loop {
                match dtls.poll_output(&mut buf_out) {
                    dimpl::Output::Packet(pkt) => {
                        if let Some(ref s) = self.socket_send {
                            if let Err(e) = s.send(pkt) {
                                return Err(ClientErrors::CantSendMsg(e));
                            }
                        } else {
                            return Err(ClientErrors::SocketNotInitialized);
                        }
                    }
                    dimpl::Output::Connected => return Ok(true),
                    dimpl::Output::Timeout(deadline) => {
                        let now = Instant::now();
                        if deadline > now {
                            std::thread::sleep(deadline - now);
                        }
                        let _ = dtls.handle_timeout(Instant::now());
                        break;
                    }
                    _ => {}
                }
            }
            if let Some(ref s) = self.socket_recev {
                match s.recv(&mut buf_in) {
                    Ok(n) => {
                        let _ = dtls.handle_packet(&buf_in[..n]);
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => return Err(ClientErrors::CantConect(e)),
                }
            }
            if handshake_start.elapsed() > handshake_timeout {
                return Ok(false);
            }
        }
    }
}
