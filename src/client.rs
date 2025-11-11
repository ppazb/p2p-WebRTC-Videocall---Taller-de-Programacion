use crate::camera::Camera;
use crate::client_errors::ClientErrors;
use crate::encoding::{DecoderRtc, EncoderRtc};
use crate::mensaje::Message;
use crate::mensaje::{Content, Signal};
use crate::rtp::RtpPackage;
use crate::sdp_protocol::*;
use crate::video_processing_errors::VideoProcessingErrors;
use opencv::core::{Mat, MatTraitConst};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use std::vec;
use std::{net::UdpSocket, thread};

#[derive(Debug)]
pub struct Client {
    socket_send: UdpSocket,
    socket_recev: UdpSocket,
    //sessions: Session,
    tx_remoto: SyncSender<Mat>,
    in_call: Arc<AtomicBool>,

    camera: Arc<Mutex<Camera>>,
}

impl Client {
    ///Constructor de cliente, requiere de un socket para generar un receptor y un emisor
    /// tambien recive channel para comunicarse con la UI.
    pub fn new(
        socket: UdpSocket,
        tx_remoto: SyncSender<Mat>,
        in_call: Arc<AtomicBool>,
        camera: Arc<Mutex<Camera>>,
    ) -> Result<Self, ClientErrors> {
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        Ok(Self {
            socket_recev: socket.try_clone().map_err(ClientErrors::CantClone)?,
            socket_send: socket,
            tx_remoto,
            in_call,

            camera,
        })
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
            .try_clone()
            .map_err(ClientErrors::CantClone)?;
        let socket_send = self
            .socket_send
            .try_clone()
            .map_err(ClientErrors::CantClone)?;
        // Thread receiver
        let tx_remoto = self.tx_remoto.clone();
        let in_call = Arc::clone(&self.in_call);

        let receiver_thread = thread::spawn(move || -> Result<(), ClientErrors> {
            Self::reciver(&socket_recev, &tx_remoto, &in_call)
        });

        // Thread sender
        let in_call = Arc::clone(&self.in_call);
        let camera = Arc::clone(&self.camera);

        let sender_thread = thread::spawn(move || -> Result<(), ClientErrors> {
            Self::sender(&socket_send, &camera, &in_call)
        });
        let handles = vec![receiver_thread, sender_thread];
        Ok(handles)
    }

    ///Levanta la camara, y se queda constantemente enviando mensajes por medio de la funcion
    /// hasta encontrarse con la señal de terminar,luego de esto enviar el mensaje de "fin" de llamada
    /// realiza este envio varias veces para asegurar que llegue a destino por estar utilizando UDP
    /// Al detectar la señal de fin de llamada le comunica esta decision a la aplicacion para un cierre correcto
    fn sender(
        socket_send: &UdpSocket,
        camera: &Arc<Mutex<Camera>>,
        in_call: &Arc<AtomicBool>,
    ) -> Result<(), ClientErrors> {
        let mut encoder = EncoderRtc::new_encoding()?;

        loop {
            // Corte de llamada y notificación remota
            if !in_call.load(Ordering::SeqCst) {
                // Genero el mensaje y lo envío repetidas veces para asegurarlo
                let msg = Message::new_msg_signalling(Signal::EndCall);

                for _ in 0..3 {
                    if let Ok(bytes) = msg.serialize()
                        && socket_send.send(&bytes).is_err()
                    {
                        eprintln!("Error enviando señal EndCall");
                    }
                }
                return Ok(());
            }
            Self::send_message(socket_send, camera, &mut encoder)?;
        }
    }

    ///Crea un decoder, y una tabla de hash y luego se queda escuchando constantemente por el socket
    /// detecta el tipo de mensaje que se recive y delega la responsabilidad a la funcion determinada
    /// pasando los recursos necesarios para permitir el funcionamiento correcto
    /// la estructura esta pensada para ser escalable y agregar funcionalidad sea un proceso simple
    /// En caso de detectar el fin de la llamada comunica esta decision a la aplicacion para un correcto cierre.
    fn reciver(
        socket_recev: &UdpSocket,
        tx_remoto: &SyncSender<Mat>,
        in_call: &Arc<AtomicBool>,
    ) -> Result<(), ClientErrors> {
        let mut decoder = DecoderRtc::new_decoder()?;
        let mut chunks_hash: HashMap<u32, Vec<(u16, Vec<u8>)>> = HashMap::new();

        loop {
            // Controlo el flag in_call para salir del thread
            if !in_call.load(Ordering::SeqCst) {
                return Ok(());
            }
            if let Some(msg) = Self::recieve_message(socket_recev) {
                //Self::process_content(msg,&mut decoder,&mut chunks_hash,& tx_remoto);
                match msg.content {
                    Content::Signal(signal) => Self::signal_handle(signal, in_call),
                    Content::Text(_text) => continue,
                    Content::Binary(media) => Self::video_handle(
                        media,
                        &mut decoder,
                        &mut chunks_hash,
                        msg.dimension.0,
                        msg.dimension.1,
                        tx_remoto,
                    ),
                }
            } else {
                continue;
            }
        }
    }

    /// Se encarga de recibir el frame de la camara y encodearlo.
    /// Luego envia el frame tanto a la UI como por mensaje de video a los clientes.
    /// Antes de enviar el mensaje se asegura de crear un paquete RTC con el mensaje serializado
    /// para esto indexea cada parte de un mismo mensaje en un bucle para su posterior remsamblaje
    fn send_message(
        socket_send: &UdpSocket,
        camera: &Arc<Mutex<Camera>>,
        encoder: &mut EncoderRtc,
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
        let encoded_frame = encoder.encode(&frame).unwrap_or_default();

        // if encoded_frame.is_empty() {
        //     return Ok(());
        // };

        let mut index_chunk: u16 = 0;
        for chunk in encoded_frame.chunks(chunk_size) {
            let rtp_package = RtpPackage::new(index_chunk, chunk.to_vec());
            let rtp_bytes = rtp_package.to_bytes();
            let video_to_send = Message::new_msg_binary(rtp_bytes, camara_width, camara_height);
            let bytes_to_send = video_to_send
                .serialize()
                .map_err(|_e| ClientErrors::SerializeBits)?;
            if let Err(e) = socket_send.send(&bytes_to_send) {
                eprintln!("Send failed: {}", e);
            }
            index_chunk = index_chunk.wrapping_add(1);
        }
        Ok(())
    }

    /// Reemsambla el manesaje recibido por socket en caso de haber sido recibido completo, lo devuelve
    /// si no se recibio nada devuelve None
    fn recieve_message(socket_recev: &UdpSocket) -> Option<Message> {
        let mut buff = vec![0u8; 60000];
        match socket_recev.recv(&mut buff) {
            Ok(0) => None,
            Ok(n) => Message::deserializable(&buff[..n]),
            Err(_) => None,
        }
    }

    ///manejo de señal, en caso de querer cortar la llamada comunica esto
    /// la funcion nos permite agregar diversos manejos de señal en caso de ser necesario
    fn signal_handle(signal: Signal, in_call: &Arc<AtomicBool>) {
        match signal {
            Signal::EndCall => {
                // El otro cliente genera la señal, se deben terminar los threads de la llamada
                in_call.store(false, Ordering::SeqCst);
            }
        }
    }

    /// Funcion encargada de maneja el video al ser recibido, utiliza el hash map para reemsamblar el frame antes de decodearlo
    /// utilizando el time stamp y el indice que se agrega al momento de encodearlo de esta manera podemos saber y agrupar que
    /// chunks de video pertenecen al mismo frame y deben ir juntas, una vez que se recive el ultimo chunk (siempre es el mas chico)
    /// se recorre todos los chunks del hash para esa clave en orden de indice( porque UDP no garantiza orden)
    /// y se envia el frame a desencodear, si el frame esta correcto, lo enviamos a la UI para su visualizacion
    /// En caso contrario continuamos, puesto que normal que nos siempre se envie un frame completo puesto que el encoder tarde mas
    /// de un ciclo en generarlo.
    /// Tambien se contempla el caso de que el frame entre en un chunk, por lo cual debe ser desencodeado inmediatamente ya que no
    /// hay "partes".
    fn video_handle(
        video_chunk: Vec<u8>,
        decoder: &mut DecoderRtc,
        chunks_hash: &mut HashMap<u32, Vec<(u16, Vec<u8>)>>,
        width: i32,
        height: i32,
        tx_remoto: &SyncSender<Mat>,
    ) {
        let chunk_size = 1200;
        if let Some(rtp) = RtpPackage::from_bytes(&video_chunk) {
            let time_stamp = rtp.time_stamp;
            let entry = chunks_hash.entry(time_stamp).or_default();
            entry.push((rtp.secuence_number, rtp.payload));

            if entry.last().map(|(_, p)| p.len()).unwrap_or(0) < chunk_size {
                // reassemble
                entry.sort_by_key(|(i, _)| *i);
                let mut full: Vec<u8> = Vec::new();
                for (_, p) in entry.drain(..) {
                    full.extend(p);
                }
                chunks_hash.remove(& time_stamp);

                if let Ok(frame) = decoder.decode(&full, width, height)
                    && tx_remoto.try_send(frame).is_err()
                {
                    // Intento enviar frame a la interfaz
                    // El envío puede fallar si ya hay un frame en el syncchannel que no se mostró todavía
                    // Esto evita también que se acumulen frames con la app minimizada
                    eprintln!("VIDEO HANDLE: UI tiene un frame sin leer!");
                }
            }
        } else {
            // not an RTP fragment, treat as full frame
            if let Ok(frame) = decoder.decode(&video_chunk, width, height)
                && tx_remoto.try_send(frame).is_err()
            {
                eprintln!("PROBLEMA envio de frame");
            }
        }
    }
}
