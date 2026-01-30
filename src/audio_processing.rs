use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

// Configuración de audio estándar para telefonía
const SAMPLE_RATE_NET: u32 = 8000;
const MAX_REMOTE_AUDIO_BUFFER_SAMPLES: usize = 1600;  //limtamos el buffer para reducir latencia

pub struct AudioSystem {
    _stream_input: cpal::Stream,
    _stream_output: cpal::Stream,
}
//implementacion de metodos de AudioSystem
impl AudioSystem {
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
            _stream_input: stream_input,
            _stream_output: stream_output,
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