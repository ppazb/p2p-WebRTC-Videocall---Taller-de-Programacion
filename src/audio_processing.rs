use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

// Configuración de audio estándar para telefonía
const SAMPLE_RATE_NET: u32 = 8000;
const MAX_REMOTE_AUDIO_BUFFER_SAMPLES: usize = 1600; // se limita la latencia a aprox. 0.2s

pub struct AudioSystem {
    // Flags de control
    capture_enabled: AtomicBool,
    mic_test_enabled: AtomicBool,

    // Configuración fija de dispositivo
    downsample_ratio: usize,
    upsample_ratio: usize,
    channels_in: usize,
    channels_out: usize,

    // Ruteo de audio en red
    tx_audio_packets: Mutex<Option<Sender<Vec<u8>>>>,
    remote_audio_buffer: Mutex<Option<Arc<Mutex<VecDeque<f32>>>>>,

    // Loopback para mic test
    loopback_buffer: Mutex<VecDeque<f32>>,
}

impl AudioSystem {
    pub fn new() -> Result<Arc<Self>, String> {
        let host = cpal::default_host(); // Referencia al audio del sistema

        // Configuración de Input
        let input_device = host
            .default_input_device() // micrófono del sistema
            .ok_or("No input device available")?;
        let supported_in = input_device
            .default_input_config() // configuración del micrófono
            .map_err(|e| e.to_string())?;
        let config_in: cpal::StreamConfig = supported_in.clone().into();
        let channels_in = config_in.channels as usize;

        // Cálculo para frenar latencia, tomamos la frecuencia de muestreo del microfono
        // y la dividimos para llegar a la frecuencia mas propia de movil 8000 hz
        let input_sample_rate = config_in.sample_rate.0;
        let downsample_ratio = (input_sample_rate / SAMPLE_RATE_NET).max(1) as usize;
        println!(
            "[AUDIO] Frecuencia del microfono: {} Hz. frecuencia rescalada: {}",
            input_sample_rate, downsample_ratio
        );

        // Configuración de Output
        let output_device = host
            .default_output_device()
            .ok_or("No output device available")?;
        let supported_out = output_device
            .default_output_config()
            .map_err(|e| e.to_string())?;
        let config_out: cpal::StreamConfig = supported_out.clone().into();
        let channels_out = config_out.channels as usize;

        // Calculamos ratio de Upsampling si salida es 48000 y red es 8000, ratio = 6
        let output_sample_rate = config_out.sample_rate.0;
        let upsample_ratio = (output_sample_rate / SAMPLE_RATE_NET).max(1) as usize;
        println!(
            "[AUDIO] Salida de audio por defecto: {} Hz. ratio de Upsampling : {}",
            output_sample_rate, upsample_ratio
        );

        // Instancia de AudioSystem
        let audio_system = Arc::new(AudioSystem {
            capture_enabled: AtomicBool::new(false),
            mic_test_enabled: AtomicBool::new(false),

            downsample_ratio,
            upsample_ratio,
            channels_in,
            channels_out,

            tx_audio_packets: Mutex::new(None),
            remote_audio_buffer: Mutex::new(None),
            loopback_buffer: Mutex::new(VecDeque::new()),
        });

        Ok(audio_system)
    }

    pub fn build_input_stream(audio: Arc<AudioSystem>) -> Result<cpal::Stream, String> {
        // Recupero información de Input
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;
        let supported = device.default_input_config().map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = supported.clone().into();

        // Matcheamos el formato del audio y armamos la función de callback correspondiente
        // Si la entrada no es f32, utilizamos una fórmula para convertilo a f32 y que sea usable por CPAL
        let stream = match supported.sample_format() {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| audio.process_input_f32(data),
                |e| eprintln!("Error audio input: {}", e),
                None,
            ),
            SampleFormat::I16 => {
                let cb = move |data: &[i16], _: &_| {
                    let tmp: Vec<f32> = data
                        .iter()
                        .map(|s| (*s as f32) / (i16::MAX as f32))
                        .collect();
                    audio.process_input_f32(&tmp);
                };
                device.build_input_stream(
                    &config,
                    cb,
                    |e| eprintln!("Error audio input: {}", e),
                    None,
                )
            }
            SampleFormat::U16 => {
                let cb = move |data: &[u16], _: &_| {
                    let tmp: Vec<f32> = data
                        .iter()
                        .map(|s| ((*s as f32) / (u16::MAX as f32)) * 2.0 - 1.0)
                        .collect();
                    audio.process_input_f32(&tmp);
                };
                device.build_input_stream(
                    &config,
                    cb,
                    |e| eprintln!("Error audio input: {}", e),
                    None,
                )
            }

            _ => {
                return Err(format!(
                    "Unsupported input sample format: {:?}",
                    supported.sample_format()
                ));
            }
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(stream)
    }

    fn process_input_f32(&self, data: &[f32]) {
        // No se procesa audio si la captura está deshabilitada
        if !self.capture_enabled.load(Ordering::Relaxed) {
            return;
        }

        // Obtenemos la cantidad de frames y creamos un vector donde guardar los chunks comprimidos
        let frames = data.len() / self.channels_in;
        let ratio = self.downsample_ratio;
        let mut compressed_chunk = Vec::with_capacity(frames / ratio + 1);

        // Bucle de downsampling, promediamos las muestras obtenidas para reconstruir el audio
        let mut i_frame = 0usize;
        while i_frame < frames {
            let mut sum = 0f32;
            let mut count = 0usize;

            for k in 0..ratio {
                let f = i_frame + k;
                if f >= frames {
                    break;
                }
                sum += data[f * self.channels_in];
                count += 1;
            }

            // Calculamos el promedio manteniendonos en el margen (1,-1)
            if count == 0 {
                break;
            }
            let avg = (sum / count as f32).clamp(-1.0, 1.0);

            // El buffer de loopback usa la misma muestra downsampleada
            if self.mic_test_enabled.load(Ordering::Relaxed)
                && let Ok(mut lb) = self.loopback_buffer.lock()
            {
                lb.push_back(avg);

                // Límite de buffer (se libera el exceso de muestras)
                if lb.len() > MAX_REMOTE_AUDIO_BUFFER_SAMPLES {
                    let excess = lb.len() - MAX_REMOTE_AUDIO_BUFFER_SAMPLES;
                    lb.drain(0..excess);
                }
            }

            // Convertimos el flotante en el rango a un entero de 16 bits redondeando
            let pcm16 = (avg * i16::MAX as f32)
                .round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;

            // Agregamos el chunk comprimido al vector, y salteamos el ratio de frames para bajar latencia
            compressed_chunk.push(g711::linear_to_alaw(pcm16));
            i_frame += ratio;
        }

        // Envío el audio por red si existe un channel
        if let Ok(lock) = self.tx_audio_packets.lock()
            && let Some(tx) = lock.as_ref()
            && !compressed_chunk.is_empty()
        {
            let _ = tx.send(compressed_chunk);
        }
    }

    pub fn build_output_stream(audio: Arc<AudioSystem>) -> Result<cpal::Stream, String> {
        // Recupero información de Output
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = supported.clone().into();

        // Matcheamos el formato del audio y armamos la función de callback correspondiente
        // El audio cargado en buffers debe poder convertirse en el tipo de dato esperado
        let stream = match supported.sample_format() {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    audio.process_output_f32(data);
                },
                |e| eprintln!("Error audio output: {}", e),
                None,
            ),
            SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    audio.process_output_i16(data);
                },
                |e| eprintln!("Error audio output: {}", e),
                None,
            ),
            SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data: &mut [u16], _| {
                    audio.process_output_u16(data);
                },
                |e| eprintln!("Error audio output: {}", e),
                None,
            ),
            _ => {
                return Err(format!(
                    "Unsupported output sample format: {:?}",
                    supported.sample_format()
                ));
            }
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(stream)
    }

    fn process_output_f32(&self, data: &mut [f32]) {
        let channels = self.channels_out;
        let frames = data.len() / channels;

        // Variables para almacenar la muestra, y cantidad de veces para repetirla
        let mut current_sample = 0.0f32;
        let mut repeat_count = 0usize;

        for frame in 0..frames {
            if repeat_count == 0 {
                current_sample = self.next_output_sample();
                repeat_count = self.upsample_ratio;
            }

            for ch in 0..channels {
                data[frame * channels + ch] = current_sample;
            }

            repeat_count -= 1;
        }
    }

    fn next_output_sample(&self) -> f32 {
        // Sacamos la muestra del frente del buffer y se devuelve para cargarlo en data
        // El buffer de mic test tiene prioridad
        if self.mic_test_enabled.load(Ordering::Relaxed)
            && let Ok(mut lb) = self.loopback_buffer.lock()
            && let Some(s) = lb.pop_front()
        {
            return s;
        }

        // Durante una llamada, saco muestras del buffer de audio remoto
        if let Ok(remote_opt) = self.remote_audio_buffer.lock()
            && let Some(remote) = remote_opt.as_ref()
            && let Ok(mut rb) = remote.lock()
        {
            // Límite de buffer (se libera el exceso de muestras)
            if rb.len() > MAX_REMOTE_AUDIO_BUFFER_SAMPLES {
                let excess = rb.len() - MAX_REMOTE_AUDIO_BUFFER_SAMPLES;
                rb.drain(0..excess);
            }

            if let Some(s) = rb.pop_front() {
                return s;
            }
        }

        // No hay datos, o los buffer no están inicializados
        0.0
    }

    fn process_output_i16(&self, data: &mut [i16]) {
        let channels = self.channels_out;
        let frames = data.len() / channels;

        // Variables para almacenar la muestra, y cantidad de veces para repetirla
        let mut current_sample = 0.0f32;
        let mut repeat_count = 0usize;

        for frame in 0..frames {
            if repeat_count == 0 {
                current_sample = self.next_output_sample();
                repeat_count = self.upsample_ratio;
            }

            for ch in 0..channels {
                data[frame * channels + ch] = (current_sample * (i16::MAX as f32))
                    .round()
                    .clamp(i16::MIN as f32, i16::MAX as f32)
                    as i16;
            }

            repeat_count -= 1;
        }
    }

    fn process_output_u16(&self, data: &mut [u16]) {
        let channels = self.channels_out;
        let frames = data.len() / channels;

        // Variables para almacenar la muestra, y cantidad de veces para repetirla
        let mut current_sample = 0.0f32;
        let mut repeat_count = 0usize;

        for frame in 0..frames {
            if repeat_count == 0 {
                current_sample = self.next_output_sample();
                repeat_count = self.upsample_ratio;
            }

            for ch in 0..channels {
                data[frame * channels + ch] = ((current_sample * 0.5 + 0.5) * (u16::MAX as f32))
                    .round()
                    .clamp(0.0, u16::MAX as f32)
                    as u16;
            }

            repeat_count -= 1;
        }
    }

    pub fn set_capture(&self, state: bool) {
        self.capture_enabled.store(state, Ordering::Relaxed);

        if !self.is_capturing() {
            // Vacío los buffers existentes
            if let Ok(mut lb) = self.loopback_buffer.lock() {
                lb.clear();
            }

            if let Ok(mut rb_opt) = self.remote_audio_buffer.lock()
                && let Some(rb) = rb_opt.as_mut()
                && let Ok(mut rb_lock) = rb.lock()
            {
                rb_lock.clear();
            }
        }
    }

    pub fn set_mic_test(&self, state: bool) {
        self.mic_test_enabled.store(state, Ordering::Relaxed);
    }

    pub fn is_capturing(&self) -> bool {
        self.capture_enabled.load(Ordering::Relaxed)
    }

    pub fn is_playing_back(&self) -> bool {
        self.mic_test_enabled.load(Ordering::Relaxed)
    }

    pub fn connect_audio(&self, tx: Sender<Vec<u8>>, remote: Arc<Mutex<VecDeque<f32>>>) {
        if let Ok(mut tx_lock) = self.tx_audio_packets.lock() {
            *tx_lock = Some(tx);
        }
        if let Ok(mut rb_lock) = self.remote_audio_buffer.lock() {
            *rb_lock = Some(remote);
        }

        self.mic_test_enabled.store(false, Ordering::Relaxed);
        self.capture_enabled.store(true, Ordering::SeqCst);
    }

    pub fn disconnect_audio(&self) {
        if let Ok(mut tx_lock) = self.tx_audio_packets.lock() {
            *tx_lock = None;
        }
        if let Ok(mut rb_lock) = self.remote_audio_buffer.lock() {
            *rb_lock = None;
        }

        self.mic_test_enabled.store(false, Ordering::Relaxed);
        self.capture_enabled.store(false, Ordering::SeqCst);
    }
}

// Módulo de compresion ULTRA SIMPLIFICIADO, se baso en g771 un estandar clasico
// pero se decidio simplificarlo completamente por cuestiones de rendmiiento
pub mod g711 {
    pub fn linear_to_alaw(pcm: i16) -> u8 {
        // Tomamos los 8 bits más significativos (truncamiento)
        //vcon esto reducimos calidad, pero permitimos una mejor velocidad
        ((pcm >> 8) & 0xFF) as u8
    }

    pub fn alaw_to_linear(alaw: u8) -> i16 {
        // Restauramos a 16 bits moviendo al byte alto
        (alaw as i16) << 8
    }
}
