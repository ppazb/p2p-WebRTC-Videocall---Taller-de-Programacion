use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

// Configuración de audio estándar para telefonía
const SAMPLE_RATE_NET: u32 = 8000;
const MAX_REMOTE_AUDIO_BUFFER_SAMPLES: usize = 1600;  //limtamos el buffer para reducir latencia

pub struct AudioSystem {
    // Streams
    //stream_input: Mutex<Option<cpal::Stream>>,
    //stream_output: Mutex<Option<cpal::Stream>>,

    // Flags de control
    capture_enabled: AtomicBool,
    mic_test_enabled: AtomicBool,

    // Configuración fija de dispositivo
    downsample_ratio: usize,
    upsample_ratio: usize,
    channels_in: usize,
    channels_out: usize,

    // Ruteo de audio (dinámico)
    tx_audio_packets: Mutex<Option<Sender<Vec<u8>>>>,
    remote_audio_buffer: Mutex<Option<Arc<Mutex<VecDeque<f32>>>>>,

    // Loopback para mic test
    loopback_buffer: Mutex<VecDeque<f32>>,
}

//implementacion de metodos de AudioSystem
impl AudioSystem {
    pub fn new_2() -> Result<Arc<Self>, String> {
        let host = cpal::default_host(); // Referencia al audio del sistema

        // Configuración de Input
        let input_device = host.default_input_device() // micrófono del sistema
            .ok_or("No input device available")?;
        let supported_in = input_device.default_input_config() // configuración del micrófono
            .map_err(|e| e.to_string())?;
        let config_in: cpal::StreamConfig = supported_in.clone().into();
        let channels_in = config_in.channels as usize;

        // Cálculo para frenar latencia, tomamos la frecuencia de muestreo del microfono
        // y la dividimos para llegar a la frecuencia mas propia de movil 8000 hz
        let input_sample_rate = config_in.sample_rate.0;
        let downsample_ratio = (input_sample_rate / SAMPLE_RATE_NET).max(1) as usize;

        // Configuración de Output
        let output_device = host.default_output_device()
            .ok_or("No output device available")?;
        let supported_out = output_device.default_output_config()
            .map_err(|e| e.to_string())?;
        let config_out: cpal::StreamConfig = supported_out.clone().into();
        let channels_out = config_out.channels as usize;

        // Calculamos ratio de Upsampling si salida es 48000 y red es 8000, ratio = 6
        let output_sample_rate = config_out.sample_rate.0;
        let upsample_ratio = (output_sample_rate / SAMPLE_RATE_NET).max(1) as usize;

        // Instancia de AudioSystem
        let audio_system = Arc::new(AudioSystem {
            //stream_input: Mutex::new(None),
            //stream_output: Mutex::new(None),

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
        
        // Construcción de streams de audio
        // let input_stream = Self::build_input_stream(
        //     Arc::clone(&audio_system),
        //     &input_device,
        //     &supported_in,
        //     &config_in,
        // )?;

        // let output_stream = Self::build_output_stream(
        //     Arc::clone(&audio_system),
        //     &output_device,
        //     &supported_out,
        //     &config_out,
        // )?;

        // input_stream.play().map_err(|e| e.to_string())?;
        // output_stream.play().map_err(|e| e.to_string())?;

        // *audio_system.stream_input.lock().unwrap() = Some(input_stream);
        // *audio_system.stream_output.lock().unwrap() = Some(output_stream);

        return Ok(audio_system);
    }

    /*
        //input de entrada matcheamos si el audio viene como f32, llamamos al metodo para crear la entrada y utilizamos el blque
        //anterior para devolver la funcion a utilizar--> camino sencillo.
        //Por el contrario si la entrada no esta como f32, utilizamos una formula para convertilo a f32 puesto que cpal solo recibe f32
        let stream_input = match supported_in.sample_format() {
            SampleFormat::F32 => {
                let cb = move |data: &[f32], _: &_| {
                    audio_cb.process_input_f32(data, channels_in);
                };
                input_device.build_input_stream(
                    &config_in, 
                    cb, 
                    |err| eprintln!("Error audio input: {}", err), 
                    None)
            }

            SampleFormat::I16 => {
                let cb = move |data: &[i16], _: &_| {
                    let tmp: Vec<f32> = data.iter()
                        .map(|s| (*s as f32) / (i16::MAX as f32)).collect();
                    audio_cb.process_input_f32(&tmp, channels_in);
                };
                input_device.build_input_stream(
                    &config_in, 
                    cb, 
                    |err| eprintln!("Error audio input: {}", err), 
                    None)
            }

            SampleFormat::U16 => {
                let cb = move |data: &[u16], info: &cpal::InputCallbackInfo| {
                    // Convert unsigned PCM to -1.0..1.0 f32 domain
                    let tmp: Vec<f32> = data.iter()
                        .map(|s| ((*s as f32) / (u16::MAX as f32)) * 2.0 - 1.0).collect();
                    audio_cb.process_input_f32(&tmp, channels_in);
                    //(make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()))(&tmp, info);
                };
                input_device.build_input_stream(
                    &config_in, 
                    cb, 
                    |err| eprintln!("Error audio input: {}", err), 
                    None)
            }

            _ => return Err(format!("Unsupported input sample format: {:?}", supported_in.sample_format())),
        }.map_err(|e| e.to_string())?;

        // input_device
        //     .build_input_stream(
        //         &config_in,
        //         input_cb,
        //         |e| eprintln!("Audio input error: {e}"),
        //         None,
        //     )
        //     .map_err(|e| e.to_string())?;

        // ETAPA 4: configuro output stream
        let output_device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let supported_out = output_device
            .default_output_config()
            .map_err(|e| e.to_string())?;

        let config_out: cpal::StreamConfig = supported_out.clone().into();
        let channels_out = config_out.channels as usize;

        let mic_test_flag = AtomicBool::clone(&mic_test_enabled);
        let tx_buffer = tx_audio_packets.clone();
        let loopback = loopback_buffer.clone();

        let output_cb = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut next_sample = || {
                if mic_test_flag.load(Ordering::Relaxed) {
                    loopback.lock().ok()?.pop_front()
                } else {
                    rx_buffer.lock().ok()?.as_ref()?.lock().ok()?.pop_front()
                }
            };

            for frame in out.chunks_mut(channels_out) {
                let s = next_sample().unwrap_or(0.0);
                for ch in frame.iter_mut() {
                    *ch = s;
                }
            }
        };

        let stream_output = output_device
            .build_output_stream(
                &config_out,
                output_cb,
                |e| eprintln!("Audio output error: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;

        // ETAPA 5: arranque
        stream_input.play().map_err(|e| e.to_string())?;
        stream_output.play().map_err(|e| e.to_string())?;

        Ok(audio_system)

    */

    pub fn build_input_stream(
        audio: Arc<AudioSystem>,
        // device: &cpal::Device,
        // supported: &cpal::SupportedStreamConfig,
        // config: &cpal::StreamConfig,
    ) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();

        // Configuración de Input
        let device = host.default_input_device() // micrófono del sistema
            .ok_or("No input device available")?;
        let supported = device.default_input_config() // configuración del micrófono
            .map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = supported.clone().into();

        //input de entrada matcheamos si el audio viene como f32, llamamos al metodo para crear la entrada y utilizamos el blque
        //anterior para devolver la funcion a utilizar--> camino sencillo.
        //Por el contrario si la entrada no esta como f32, utilizamos una formula para convertilo a f32 puesto que cpal solo recibe f32

        //let audio2 = Arc::clone(&audio);
        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| audio.process_input_f32(data),
                    |e| eprintln!("Error audio input: {}", e),
                    None,
                )
            }
            SampleFormat::I16 => {
                let cb = move |data: &[i16], _: &_| {
                    let tmp: Vec<f32> = data.iter()
                        .map(|s| (*s as f32) / (i16::MAX as f32)).collect();
                    audio.process_input_f32(&tmp);
                };
                device.build_input_stream(
                    &config, 
                    cb, 
                    |e| eprintln!("Error audio input: {}", e), 
                    None)
            }
            SampleFormat::U16 => {
                let cb = move |data: &[u16], _: &_| {
                    let tmp: Vec<f32> = data.iter()
                        .map(|s| ((*s as f32) / (u16::MAX as f32)) * 2.0 - 1.0).collect();
                    audio.process_input_f32(&tmp);
                };
                device.build_input_stream(
                    &config, 
                    cb, 
                    |e| eprintln!("Error audio input: {}", e), 
                    None)
            }

            _ => return Err(format!("Unsupported input sample format: {:?}", supported.sample_format())),
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
                if f >= frames { break; }
                sum += data[f * self.channels_in];
                count += 1;
            }

            // Calculamos el promedio manteniendonos en el margen (1,-1)
            if count == 0 { break; }
            let avg = (sum / count as f32).clamp(-1.0, 1.0);

            // Loopback usa la MISMA muestra downsampleada
            if self.mic_test_enabled.load(Ordering::Relaxed) {
                if let Ok(mut lb) = self.loopback_buffer.lock() {
                    lb.push_back(avg);

                    // límite de buffer (clave)
                    if lb.len() > MAX_REMOTE_AUDIO_BUFFER_SAMPLES {
                        let excess = lb.len() - MAX_REMOTE_AUDIO_BUFFER_SAMPLES;
                        lb.drain(0..excess);
                    }
                }
            }

            // Convertimos el flotante en el rango a un entero de 16 bits redondeando
            let pcm16 = (avg * i16::MAX as f32).round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;

            // Agregamos el chunk comprimido al vector, y salteamos el ratio de frames para bajar latencia
            compressed_chunk.push(g711::linear_to_alaw(pcm16));
            i_frame += ratio;
        }

        // Envío el audio por red si existe un channel
        if let Ok(lock) = self.tx_audio_packets.lock() {
            if let Some(tx) = lock.as_ref() {
                if !compressed_chunk.is_empty() {
                    let _ = tx.send(compressed_chunk);
                }
            }
        }

        

        // Vuelvo a cargar la data para test de micrófono (loopback)
        // if self.mic_test_enabled.load(Ordering::Relaxed) {
        //     if let Ok(mut lb) = self.loopback_buffer.lock() {
        //         for &s in data.iter().step_by(self.channels_in) {
        //             lb.push_back(s);
        //         }
        //     }
        // }

        // OTRA VERSIÓN
        // Tomamos solo el canal 0 (mono)
        //     let samples = data.chunks(channels_in).map(|f| f[0]);

        //     // Flag para reproducir audio propio localmente
        //     if mic_test_flag.load(Ordering::Relaxed) {
        //         if let Ok(mut buf) = loopback.lock() {
        //             buf.extend(samples);
        //         }
        //         return;
        //     }

    }





    pub fn build_output_stream(
        audio: Arc<AudioSystem>,
    ) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();

        // Configuración de Output
        let device = host.default_output_device()
            .ok_or("No output device available")?;
        let supported = device.default_output_config()
            .map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = supported.clone().into();

        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _| {
                        audio.process_output_f32(data);
                    },
                    |e| eprintln!("output error: {}", e),
                    None,
                )
            }
            // SampleFormat::I16 => {
            //     let cb = move |data: &mut [i16], _: &_| {
            //         let mut tmp: Vec<f32> = data.iter()
            //             .map(|s| (*s as f32) / (i16::MAX as f32)).collect();
            //         audio.process_output_f32(&mut tmp);
            //     };
            //     device.build_output_stream(
            //         &config, 
            //         cb, 
            //         |e| eprintln!("Error audio output: {}", e), 
            //         None
            //     )
            // }
            // SampleFormat::U16 => {
            //     let cb = move |data: &mut [u16], _: &_| {
            //         let mut tmp: Vec<f32> = data.iter()
            //             .map(|s| ((*s as f32) / (u16::MAX as f32)) * 2.0 - 1.0).collect();
            //         audio.process_output_f32(&mut tmp);
            //     };
            //     device.build_input_stream(
            //         &config, 
            //         cb, 
            //         |e| eprintln!("Error audio input: {}", e), 
            //         None
            //     )
            // }
            SampleFormat::I16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _| {
                        audio.process_output_i16(data);
                    },
                    |e| eprintln!("output error: {}", e),
                    None,
                )
            }
            SampleFormat::U16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _| {
                        audio.process_output_u16(data);
                    },
                    |e| eprintln!("output error: {}", e),
                    None,
                )
            }
            _ => return Err(format!("Unsupported output sample format: {:?}", supported.sample_format())),
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(stream)
    }

    fn process_output_i16(&self, data: &mut [i16]) {
        let mut temp = vec![0.0f32; data.len()];
        self.process_output_f32(&mut temp);

        for (dst, src) in data.iter_mut().zip(temp.iter()) {
            *dst = Self::f32_to_i16(*src);
        }
    }

    fn f32_to_i16(x: f32) -> i16 {
        (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
    }

    fn process_output_u16(&self, data: &mut [u16]) {
        let mut temp = vec![0.0f32; data.len()];
        self.process_output_f32(&mut temp);

        for (dst, src) in data.iter_mut().zip(temp.iter()) {
            *dst = Self::f32_to_u16(*src);
        }
    }

    fn f32_to_u16(x: f32) -> u16 {
        ((x.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16
    }

    fn process_output_f32(&self, data: &mut [f32]) {
        let channels = self.channels_out as usize;
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
        // Sacamos la muestra del frente del buffer
        let max_samples = MAX_REMOTE_AUDIO_BUFFER_SAMPLES; // usamos el valor definido para sacar el ratio

        // 1️⃣ mic test tiene prioridad
        if self.mic_test_enabled.load(Ordering::Relaxed) {
            if let Ok(mut lb) = self.loopback_buffer.lock() {
                if let Some(s) = lb.pop_front() {
                    return s;
                }
            }
        }

        // 2️⃣ audio remoto (llamada)
        if let Ok(remote_opt) = self.remote_audio_buffer.lock() {
            if let Some(remote) = remote_opt.as_ref() {
                if let Ok(mut rb) = remote.lock() {
                    //linea critica, donde analisamos si el buffer que usamos esta rebalsado o no
                    //si esta rebalsado liberamos lo mas viejo con drain hasta que vuevla al limite fijado
                    //de esta forma aseguramos mantener una latencia constante
                    if rb.len() > max_samples {
                        let excess = rb.len() - max_samples;
                        rb.drain(0..excess);
                    }

                    if let Some(s) = rb.pop_front() {
                        return s;
                    }
                }
            }
        }

        // No hay datos, o los buffer no están inicializados
        0.0
    }

    /*
    let max_samples = MAX_REMOTE_AUDIO_BUFFER_SAMPLES; // usamos el valor definido para sacar el ratio
        let stream_output = match supported_out.sample_format() {
            SampleFormat::F32 => {
                let mut current_sample_val = 0.0f32;
                let mut sample_repeat_count = 0usize;  //contador para saber cuantas veces repetir la muestra
                let cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    match remote_audio_buffer.lock() {
                        Ok(mut buffer) => {
                            //linea critica, donde analisamos si el buffer que usamos esta rebalsado o no
                            //si esta rebalsado liberamos lo mas viejo con drain hasta que vuevla al limite fijado
                            //de esta forma aseguramos mantener una latencia constante
                            if buffer.len() > max_samples {
                                let excess = buffer.len() - max_samples;
                                buffer.drain(0..excess);
                            }
                            let channels = config_out.channels as usize;
                            let frames = data.len() / channels;
                            let mut frame_idx = 0usize;
                            while frame_idx < frames {
                                if sample_repeat_count == 0 { // si el contador esta en 0, sacamos del frente del buffer y reiniciamos
                                    if let Some(new_val) = buffer.pop_front() {
                                        current_sample_val = new_val;
                                        sample_repeat_count = upsample_ratio;
                                    } else {
                                        current_sample_val = 0.0; //no hay datos, el buffer no devolvio some
                                    }
                                }
                                for ch in 0..channels {
                                    data[frame_idx * channels + ch] = current_sample_val;
                                }
                                frame_idx += 1;
                                if sample_repeat_count > 0 {
                                    sample_repeat_count -= 1;
                                }
                            }
                        }
                        Err(_) => {
                            for s in data.iter_mut() { *s = 0.0; }
                        }
                    }
                };
                //devolvemos un ouput stream con los parametros creados y la data obtenida
                output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
            }
            //Otros casos de salida de audio, I16 y U16 para convertirlos a F32 y poder trabajar con Cpal 
            SampleFormat::I16 => {
                let mut current_sample_val = 0.0f32;
                let mut sample_repeat_count = 0usize;
                let cb = move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    match remote_audio_buffer.lock() {
                        Ok(mut buffer) => {
                            if buffer.len() > max_samples {
                                let excess = buffer.len() - max_samples;
                                buffer.drain(0..excess);
                            }
                            let channels = config_out.channels as usize;
                            let frames = data.len() / channels;
                            let mut frame_idx = 0usize;
                            while frame_idx < frames {
                                if sample_repeat_count == 0 {
                                    if let Some(new_val) = buffer.pop_front() {
                                        current_sample_val = new_val;
                                        sample_repeat_count = upsample_ratio;
                                    } else {
                                        current_sample_val = 0.0;
                                    }
                                }
                                for ch in 0..channels {
                                    data[frame_idx * channels + ch] = (current_sample_val * (i16::MAX as f32)).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                                }
                                frame_idx += 1;
                                if sample_repeat_count > 0 { sample_repeat_count -= 1; }
                            }
                        }
                        Err(_) => { for s in data.iter_mut() { *s = 0; } }
                    }
                };
                output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
            }
            SampleFormat::U16 => {
                let mut current_sample_val = 0.0f32;
                let mut sample_repeat_count = 0usize;
                let cb = move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    match remote_audio_buffer.lock() {
                        Ok(mut buffer) => {
                            if buffer.len() > max_samples {
                                let excess = buffer.len() - max_samples;
                                buffer.drain(0..excess);
                            }
                            let channels = config_out.channels as usize;
                            let frames = data.len() / channels;
                            let mut frame_idx = 0usize;
                            while frame_idx < frames {
                                if sample_repeat_count == 0 {
                                    if let Some(new_val) = buffer.pop_front() {
                                        current_sample_val = new_val;
                                        sample_repeat_count = upsample_ratio;
                                    } else {
                                        current_sample_val = 0.0;
                                    }
                                }
                                let u = ((current_sample_val * 0.5 + 0.5) * (u16::MAX as f32)).round().clamp(0.0, u16::MAX as f32) as u16;
                                for ch in 0..channels {
                                    data[frame_idx * channels + ch] = u;
                                }
                                frame_idx += 1;
                                if sample_repeat_count > 0 { sample_repeat_count -= 1; }
                            }
                        }
                        Err(_) => { for s in data.iter_mut() { *s = 0; } }
                    }
                };
                output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
            }
            // si no matchea con nignuno de los formatos, entonces no es soportado por el programa, devolvemos un error
            _ => return Err(format!("Unsupported output sample format: {:?}", supported_out.sample_format())),
        }.map_err(|e| e.to_string())?;
    */






























    

    pub fn enable_capture(&self) {
        self.capture_enabled.store(true, Ordering::Relaxed);
        self.mic_test_enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable_capture(&self) {
        self.capture_enabled.store(false, Ordering::Relaxed);
        self.mic_test_enabled.store(false, Ordering::Relaxed);
    }

    pub fn is_capturing(&self) -> bool {
        self.capture_enabled.load(Ordering::Relaxed)
    }

        // let make_input_callback = move |channels: u16, downsample_ratio: usize, tx_audio_packets: Sender<Vec<u8>>| {
        //     move |data: &[f32], _: &cpal::InputCallbackInfo| {
        //         //cuantos frames vamos a tener y creamos una variable donde guardar los chunks
        //         let frames = data.len() / channels as usize;
        //         let mut compressed_chunk = Vec::with_capacity(frames / downsample_ratio + 1);
        //         let mut i_frame = 0usize;
        //         //bucle de de downsampling, promediamos las muestras obtenidas para reconstruir el audio 
        //         while i_frame < frames {
        //             let mut sum = 0f32;
        //             let mut count = 0usize;
        //             for k in 0..downsample_ratio {
        //                 let f = i_frame + k;
        //                 if f >= frames { break; }
        //                 let sample = data[f * channels as usize];
        //                 sum += sample;
        //                 count += 1;
        //             }
        //             //calculamos el promedio manteniendonos en el marge (1,-1)
        //             if count == 0 { break; }
        //             let avg = (sum / count as f32).clamp(-1.0, 1.0);
        //             //convertimos el flotante en el rango a un entero de 16 bits redondeando
        //             let pcm16 = (avg * (i16::MAX as f32)).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        //             let compressed = g711::linear_to_alaw(pcm16); // llamamos al modulo de compresion
        //             compressed_chunk.push(compressed); //pusheamos al vector el chunk compreso
        //             i_frame += downsample_ratio; //salteamos el ratio de frame pasosos para reducir latencia
        //         }
        //         //enviamos el audio por el chanel
        //         if !compressed_chunk.is_empty() {
        //             let _ = tx_audio_packets.send(compressed_chunk);
        //         }
        //     }
        // };

        // //input de entrada matcheamos si el audio viene como f32, llamamos al metodo para crear la entrada y utilizamos el blque
        // //anterior para devolver la funcion a utilizar--> camino sencillo.
        // //Por el contrario si la entrada no esta como f32, utilizamos una formula para convertilo a f32 puesto que cpal solo recibe f32
        // let stream_input = match supported_in.sample_format() {
        //     SampleFormat::F32 => input_device.build_input_stream(&config_in, make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()), |err| eprintln!("Error audio input: {}", err), None),
        //     SampleFormat::I16 => {
        //         let cb = move |data: &[i16], info: &cpal::InputCallbackInfo| {
        //             // convert to f32 slice on the fly into a temporary vec and call the f32 callback
        //             let mut tmp: Vec<f32> = data.iter().map(|s| (*s as f32) / (i16::MAX as f32)).collect();
        //             (make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()))(&tmp, info);
        //         };
        //         input_device.build_input_stream(&config_in, cb, |err| eprintln!("Error audio input: {}", err), None)
        //     }
        //     SampleFormat::U16 => {
        //         let cb = move |data: &[u16], info: &cpal::InputCallbackInfo| {
        //             // Convert unsigned PCM to -1.0..1.0 f32 domain
        //             let mut tmp: Vec<f32> = data.iter().map(|s| ((*s as f32) / (u16::MAX as f32)) * 2.0 - 1.0).collect();
        //             (make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()))(&tmp, info);
        //         };
        //         input_device.build_input_stream(&config_in, cb, |err| eprintln!("Error audio input: {}", err), None)
        //     }
        //     _ => return Err(format!("Unsupported input sample format: {:?}", supported_in.sample_format())),
        // }.map_err(|e| e.to_string())?;







        // //Configuracion Output del audio de sistema
        // let output_device = host.default_output_device()
        //     .ok_or("No output device available")?;
        // //tomamos la configuracion por defecto
        // let supported_out = output_device
        //     .default_output_config()
        //     .map_err(|e| e.to_string())?;
        // let config_out: cpal::StreamConfig = supported_out.clone().into();

        // // Calculamos ratio de Upsampling si salida es 48000 y red es 8000, ratio = 6
        // let output_sample_rate = config_out.sample_rate.0;
        // let upsample_ratio = (output_sample_rate / SAMPLE_RATE_NET).max(1) as usize;
        
        // println!("[AUDIO] Salida de audio por defecto: {} Hz. ratio de Upsampling : {}", output_sample_rate, upsample_ratio);
        // let max_samples = MAX_REMOTE_AUDIO_BUFFER_SAMPLES; // usamos el valor definido para sacar el ratio


        // // ETAPA 2: creación de streams
        // //input de entrada matcheamos si el audio viene como f32, llamamos al metodo para crear la entrada y utilizamos el blque
        // //anterior para devolver la funcion a utilizar--> camino sencillo.
        // //Por el contrario si la entrada no esta como f32, utilizamos una formula para convertilo a f32 puesto que cpal solo recibe f32
        // let stream_input = match supported_in.sample_format() {
        //     SampleFormat::F32 => input_device.build_input_stream(&config_in, make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()), |err| eprintln!("Error audio input: {}", err), None),
        //     SampleFormat::I16 => {
        //         let cb = move |data: &[i16], info: &cpal::InputCallbackInfo| {
        //             // convert to f32 slice on the fly into a temporary vec and call the f32 callback
        //             let mut tmp: Vec<f32> = data.iter().map(|s| (*s as f32) / (i16::MAX as f32)).collect();
        //             (make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()))(&tmp, info);
        //         };
        //         input_device.build_input_stream(&config_in, cb, |err| eprintln!("Error audio input: {}", err), None)
        //     }
        //     SampleFormat::U16 => {
        //         let cb = move |data: &[u16], info: &cpal::InputCallbackInfo| {
        //             // Convert unsigned PCM to -1.0..1.0 f32 domain
        //             let mut tmp: Vec<f32> = data.iter().map(|s| ((*s as f32) / (u16::MAX as f32)) * 2.0 - 1.0).collect();
        //             (make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()))(&tmp, info);
        //         };
        //         input_device.build_input_stream(&config_in, cb, |err| eprintln!("Error audio input: {}", err), None)
        //     }
        //     _ => return Err(format!("Unsupported input sample format: {:?}", supported_in.sample_format())),
        // }.map_err(|e| e.to_string())?;

        // let stream_output = match supported_out.sample_format() {
        //     SampleFormat::F32 => {
        //         let mut current_sample_val = 0.0f32;
        //         let mut sample_repeat_count = 0usize;  //contador para saber cuantas veces repetir la muestra
        //         let cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        //             match remote_audio_buffer.lock() {
        //                 Ok(mut buffer) => {
        //                     //linea critica, donde analisamos si el buffer que usamos esta rebalsado o no
        //                     //si esta rebalsado liberamos lo mas viejo con drain hasta que vuevla al limite fijado
        //                     //de esta forma aseguramos mantener una latencia constante
        //                     if buffer.len() > max_samples {
        //                         let excess = buffer.len() - max_samples;
        //                         buffer.drain(0..excess);
        //                     }
        //                     let channels = config_out.channels as usize;
        //                     let frames = data.len() / channels;
        //                     let mut frame_idx = 0usize;
        //                     while frame_idx < frames {
        //                         if sample_repeat_count == 0 { // si el contador esta en 0, sacamos del frente del buffer y reiniciamos
        //                             if let Some(new_val) = buffer.pop_front() {
        //                                 current_sample_val = new_val;
        //                                 sample_repeat_count = upsample_ratio;
        //                             } else {
        //                                 current_sample_val = 0.0; //no hay datos, el buffer no devolvio some
        //                             }
        //                         }
        //                         for ch in 0..channels {
        //                             data[frame_idx * channels + ch] = current_sample_val;
        //                         }
        //                         frame_idx += 1;
        //                         if sample_repeat_count > 0 {
        //                             sample_repeat_count -= 1;
        //                         }
        //                     }
        //                 }
        //                 Err(_) => {
        //                     for s in data.iter_mut() { *s = 0.0; }
        //                 }
        //             }
        //         };
        //         //devolvemos un ouput stream con los parametros creados y la data obtenida
        //         output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
        //     }
        //     //Otros casos de salida de audio, I16 y U16 para convertirlos a F32 y poder trabajar con Cpal 
        //     SampleFormat::I16 => {
        //         let mut current_sample_val = 0.0f32;
        //         let mut sample_repeat_count = 0usize;
        //         let cb = move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
        //             match remote_audio_buffer.lock() {
        //                 Ok(mut buffer) => {
        //                     if buffer.len() > max_samples {
        //                         let excess = buffer.len() - max_samples;
        //                         buffer.drain(0..excess);
        //                     }
        //                     let channels = config_out.channels as usize;
        //                     let frames = data.len() / channels;
        //                     let mut frame_idx = 0usize;
        //                     while frame_idx < frames {
        //                         if sample_repeat_count == 0 {
        //                             if let Some(new_val) = buffer.pop_front() {
        //                                 current_sample_val = new_val;
        //                                 sample_repeat_count = upsample_ratio;
        //                             } else {
        //                                 current_sample_val = 0.0;
        //                             }
        //                         }
        //                         for ch in 0..channels {
        //                             data[frame_idx * channels + ch] = (current_sample_val * (i16::MAX as f32)).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        //                         }
        //                         frame_idx += 1;
        //                         if sample_repeat_count > 0 { sample_repeat_count -= 1; }
        //                     }
        //                 }
        //                 Err(_) => { for s in data.iter_mut() { *s = 0; } }
        //             }
        //         };
        //         output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
        //     }
        //     SampleFormat::U16 => {
        //         let mut current_sample_val = 0.0f32;
        //         let mut sample_repeat_count = 0usize;
        //         let cb = move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
        //             match remote_audio_buffer.lock() {
        //                 Ok(mut buffer) => {
        //                     if buffer.len() > max_samples {
        //                         let excess = buffer.len() - max_samples;
        //                         buffer.drain(0..excess);
        //                     }
        //                     let channels = config_out.channels as usize;
        //                     let frames = data.len() / channels;
        //                     let mut frame_idx = 0usize;
        //                     while frame_idx < frames {
        //                         if sample_repeat_count == 0 {
        //                             if let Some(new_val) = buffer.pop_front() {
        //                                 current_sample_val = new_val;
        //                                 sample_repeat_count = upsample_ratio;
        //                             } else {
        //                                 current_sample_val = 0.0;
        //                             }
        //                         }
        //                         let u = ((current_sample_val * 0.5 + 0.5) * (u16::MAX as f32)).round().clamp(0.0, u16::MAX as f32) as u16;
        //                         for ch in 0..channels {
        //                             data[frame_idx * channels + ch] = u;
        //                         }
        //                         frame_idx += 1;
        //                         if sample_repeat_count > 0 { sample_repeat_count -= 1; }
        //                     }
        //                 }
        //                 Err(_) => { for s in data.iter_mut() { *s = 0; } }
        //             }
        //         };
        //         output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
        //     }
        //     // si no matchea con nignuno de los formatos, entonces no es soportado por el programa, devolvemos un error
        //     _ => return Err(format!("Unsupported output sample format: {:?}", supported_out.sample_format())),
        // }.map_err(|e| e.to_string())?;

        // // ETAPA 3: creación de buffers vacíos
        // let tx_audio_packets = 

    pub fn start_call(
        &self,
        tx: Sender<Vec<u8>>,
        remote: Arc<Mutex<VecDeque<f32>>>,
    ) {
        *self.tx_audio_packets.lock().unwrap() = Some(tx);
        *self.remote_audio_buffer.lock().unwrap() = Some(remote);
    
        self.mic_test_enabled.store(false, Ordering::Relaxed);
        self.capture_enabled.store(true, Ordering::SeqCst);
    }

    pub fn end_call(&self) {
        *self.tx_audio_packets.lock().unwrap() = None;
        *self.remote_audio_buffer.lock().unwrap() = None;

        self.mic_test_enabled.store(false, Ordering::Relaxed);
        self.capture_enabled.store(false, Ordering::SeqCst);
    }





    
    pub fn new(
        tx_audio_packets: Sender<Vec<u8>>, //canal que usaremos para comunicarnos entre hilos por ej: client.rs 
        remote_audio_buffer: Arc<Mutex<VecDeque<f32>>> // El audio que nos llega desde el otro cliente
    ) -> Result<Self, String> {
        let host = cpal::default_host(); //nuestra referencia al audio del sistema
        let input_device = host.default_input_device() //tomamos el microfono
            .ok_or("No input device available")?;
        //tomamos los parametros del microfono 
        let supported_in = input_device
            .default_input_config()
            .map_err(|e| e.to_string())?;
        let config_in: cpal::StreamConfig = supported_in.clone().into();
        //calculo para frenar latencia, tomamos la frecuencia de muestreo del microfono
        //y la dividimos para llegar a la frecuncia mas propia de movil 8000 hz
        let input_sample_rate = config_in.sample_rate.0;
        let downsample_ratio = (input_sample_rate / SAMPLE_RATE_NET).max(1) as usize;

        println!("[AUDIO] Frecuencia del microfono: {} Hz. frecuencia rescalada: {}", input_sample_rate, downsample_ratio);

        //convertimos la entrada de audio en lo que necesitamos para procesarla
        let make_input_callback = move |channels: u16, downsample_ratio: usize, tx_audio_packets: Sender<Vec<u8>>| {
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                //cuantos frames vamos a tener y creamos una variable donde guardar los chunks
                let frames = data.len() / channels as usize;
                let mut compressed_chunk = Vec::with_capacity(frames / downsample_ratio + 1);
                let mut i_frame = 0usize;
                //bucle de de downsampling, promediamos las muestras obtenidas para reconstruir el audio 
                while i_frame < frames {
                    let mut sum = 0f32;
                    let mut count = 0usize;
                    for k in 0..downsample_ratio {
                        let f = i_frame + k;
                        if f >= frames { break; }
                        let sample = data[f * channels as usize];
                        sum += sample;
                        count += 1;
                    }
                    //calculamos el promedio manteniendonos en el marge (1,-1)
                    if count == 0 { break; }
                    let avg = (sum / count as f32).clamp(-1.0, 1.0);
                    //convertimos el flotante en el rango a un entero de 16 bits redondeando
                    let pcm16 = (avg * (i16::MAX as f32)).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    let compressed = g711::linear_to_alaw(pcm16); // llamamos al modulo de compresion
                    compressed_chunk.push(compressed); //pusheamos al vector el chunk compreso
                    i_frame += downsample_ratio; //salteamos el ratio de frame pasosos para reducir latencia
                }
                //enviamos el audio por el chanel
                if !compressed_chunk.is_empty() {
                    let _ = tx_audio_packets.send(compressed_chunk);
                }
            }
        };

        //input de entrada matcheamos si el audio viene como f32, llamamos al metodo para crear la entrada y utilizamos el blque
        //anterior para devolver la funcion a utilizar--> camino sencillo.
        //Por el contrario si la entrada no esta como f32, utilizamos una formula para convertilo a f32 puesto que cpal solo recibe f32
        let stream_input = match supported_in.sample_format() {
            SampleFormat::F32 => input_device.build_input_stream(&config_in, make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()), |err| eprintln!("Error audio input: {}", err), None),
            SampleFormat::I16 => {
                let cb = move |data: &[i16], info: &cpal::InputCallbackInfo| {
                    // convert to f32 slice on the fly into a temporary vec and call the f32 callback
                    let mut tmp: Vec<f32> = data.iter().map(|s| (*s as f32) / (i16::MAX as f32)).collect();
                    (make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()))(&tmp, info);
                };
                input_device.build_input_stream(&config_in, cb, |err| eprintln!("Error audio input: {}", err), None)
            }
            SampleFormat::U16 => {
                let cb = move |data: &[u16], info: &cpal::InputCallbackInfo| {
                    // Convert unsigned PCM to -1.0..1.0 f32 domain
                    let mut tmp: Vec<f32> = data.iter().map(|s| ((*s as f32) / (u16::MAX as f32)) * 2.0 - 1.0).collect();
                    (make_input_callback(config_in.channels, downsample_ratio, tx_audio_packets.clone()))(&tmp, info);
                };
                input_device.build_input_stream(&config_in, cb, |err| eprintln!("Error audio input: {}", err), None)
            }
            _ => return Err(format!("Unsupported input sample format: {:?}", supported_in.sample_format())),
        }.map_err(|e| e.to_string())?;

        //Configuracion Output del audio de sistema
        let output_device = host.default_output_device()
            .ok_or("No output device available")?;
        //tomamos la configuracion por defecto
        let supported_out = output_device
            .default_output_config()
            .map_err(|e| e.to_string())?;
        let config_out: cpal::StreamConfig = supported_out.clone().into();

        // Calculamos ratio de Upsampling si salida es 48000 y red es 8000, ratio = 6
        let output_sample_rate = config_out.sample_rate.0;
        let upsample_ratio = (output_sample_rate / SAMPLE_RATE_NET).max(1) as usize;
        
        println!("[AUDIO] Salida de audio por defecto: {} Hz. ratio de Upsampling : {}", output_sample_rate, upsample_ratio);
        let max_samples = MAX_REMOTE_AUDIO_BUFFER_SAMPLES; // usamos el valor definido para sacar el ratio
        let stream_output = match supported_out.sample_format() {
            SampleFormat::F32 => {
                let mut current_sample_val = 0.0f32;
                let mut sample_repeat_count = 0usize;  //contador para saber cuantas veces repetir la muestra
                let cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    match remote_audio_buffer.lock() {
                        Ok(mut buffer) => {
                            //linea critica, donde analisamos si el buffer que usamos esta rebalsado o no
                            //si esta rebalsado liberamos lo mas viejo con drain hasta que vuevla al limite fijado
                            //de esta forma aseguramos mantener una latencia constante
                            if buffer.len() > max_samples {
                                let excess = buffer.len() - max_samples;
                                buffer.drain(0..excess);
                            }
                            let channels = config_out.channels as usize;
                            let frames = data.len() / channels;
                            let mut frame_idx = 0usize;
                            while frame_idx < frames {
                                if sample_repeat_count == 0 { // si el contador esta en 0, sacamos del frente del buffer y reiniciamos
                                    if let Some(new_val) = buffer.pop_front() {
                                        current_sample_val = new_val;
                                        sample_repeat_count = upsample_ratio;
                                    } else {
                                        current_sample_val = 0.0; //no hay datos, el buffer no devolvio some
                                    }
                                }
                                for ch in 0..channels {
                                    data[frame_idx * channels + ch] = current_sample_val;
                                }
                                frame_idx += 1;
                                if sample_repeat_count > 0 {
                                    sample_repeat_count -= 1;
                                }
                            }
                        }
                        Err(_) => {
                            for s in data.iter_mut() { *s = 0.0; }
                        }
                    }
                };
                //devolvemos un ouput stream con los parametros creados y la data obtenida
                output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
            }
            //Otros casos de salida de audio, I16 y U16 para convertirlos a F32 y poder trabajar con Cpal 
            SampleFormat::I16 => {
                let mut current_sample_val = 0.0f32;
                let mut sample_repeat_count = 0usize;
                let cb = move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    match remote_audio_buffer.lock() {
                        Ok(mut buffer) => {
                            if buffer.len() > max_samples {
                                let excess = buffer.len() - max_samples;
                                buffer.drain(0..excess);
                            }
                            let channels = config_out.channels as usize;
                            let frames = data.len() / channels;
                            let mut frame_idx = 0usize;
                            while frame_idx < frames {
                                if sample_repeat_count == 0 {
                                    if let Some(new_val) = buffer.pop_front() {
                                        current_sample_val = new_val;
                                        sample_repeat_count = upsample_ratio;
                                    } else {
                                        current_sample_val = 0.0;
                                    }
                                }
                                for ch in 0..channels {
                                    data[frame_idx * channels + ch] = (current_sample_val * (i16::MAX as f32)).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                                }
                                frame_idx += 1;
                                if sample_repeat_count > 0 { sample_repeat_count -= 1; }
                            }
                        }
                        Err(_) => { for s in data.iter_mut() { *s = 0; } }
                    }
                };
                output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
            }
            SampleFormat::U16 => {
                let mut current_sample_val = 0.0f32;
                let mut sample_repeat_count = 0usize;
                let cb = move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    match remote_audio_buffer.lock() {
                        Ok(mut buffer) => {
                            if buffer.len() > max_samples {
                                let excess = buffer.len() - max_samples;
                                buffer.drain(0..excess);
                            }
                            let channels = config_out.channels as usize;
                            let frames = data.len() / channels;
                            let mut frame_idx = 0usize;
                            while frame_idx < frames {
                                if sample_repeat_count == 0 {
                                    if let Some(new_val) = buffer.pop_front() {
                                        current_sample_val = new_val;
                                        sample_repeat_count = upsample_ratio;
                                    } else {
                                        current_sample_val = 0.0;
                                    }
                                }
                                let u = ((current_sample_val * 0.5 + 0.5) * (u16::MAX as f32)).round().clamp(0.0, u16::MAX as f32) as u16;
                                for ch in 0..channels {
                                    data[frame_idx * channels + ch] = u;
                                }
                                frame_idx += 1;
                                if sample_repeat_count > 0 { sample_repeat_count -= 1; }
                            }
                        }
                        Err(_) => { for s in data.iter_mut() { *s = 0; } }
                    }
                };
                output_device.build_output_stream(&config_out, cb, |err| eprintln!("Error audio output: {}", err), None)
            }
            // si no matchea con nignuno de los formatos, entonces no es soportado por el programa, devolvemos un error
            _ => return Err(format!("Unsupported output sample format: {:?}", supported_out.sample_format())),
        }.map_err(|e| e.to_string())?;
        // ejecutamos el audio de salida y el audio de entrada
        stream_input.play().map_err(|e| e.to_string())?;
        stream_output.play().map_err(|e| e.to_string())?;

        Ok(Self {
            //_stream_input: stream_input,
            //_stream_output: stream_output,
            capture_enabled: todo!(),
            mic_test_enabled: todo!(),
            tx_audio_packets: todo!(),
            remote_audio_buffer: todo!(),
            loopback_buffer: todo!(),
            downsample_ratio,
            upsample_ratio,
            channels_in: todo!(),
            channels_out: todo!(),
        })
    }
}

//Modulo de compresion ULTRA SIMPLIFICIADO, se baso en g771 un standar clasico
//pero se decidio simplificarlos completamente por cuestiones de rendmiiento
pub mod g711 {
    pub fn linear_to_alaw(pcm: i16) -> u8 {
        // Tomamos los 8 bits más significativos (truncamiento)
        //con esto reducimos calidad, pero permitimos una mejor velocidad
        ((pcm >> 8) & 0xFF) as u8
    }

    pub fn alaw_to_linear(alaw: u8) -> i16 {
        // Restauramos a 16 bits moviendo al byte alto
        (alaw as i16) << 8
    }
}



        // Callback de input
        // let input_cb = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        //     // Flag para controlar la toma de audio
        //     if !capture_flag.load(Ordering::Relaxed) {
        //         return;
        //     }

        //     // Tomamos solo el canal 0 (mono)
        //     let samples = data.chunks(channels_in).map(|f| f[0]);

        //     // Flag para reproducir audio propio localmente
        //     if mic_test_flag.load(Ordering::Relaxed) {
        //         if let Ok(mut buf) = loopback.lock() {
        //             buf.extend(samples);
        //         }
        //         return;
        //     }

        //     // Intenta enviar audio por el channel al otro cliente
        //     if let Ok(tx_opt) = tx_buffer.lock() {
        //         if let Some(tx) = tx_opt.as_ref() {
        //             let mut pkt = Vec::new();
        //             for s in samples {
        //                 let pcm = (s * i16::MAX as f32)
        //                     .round()
        //                     .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        //                 pkt.push(g711::linear_to_alaw(pcm));
        //             }
        //             let _ = tx.send(pkt);
        //         }
        //     }
        // };