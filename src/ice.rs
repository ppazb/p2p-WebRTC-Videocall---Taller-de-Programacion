use crate::ice_errors::IceErrors;
use crate::log_c;
use std::net::UdpSocket;
// Lista de códecs en orden de prioridad
const PRIORIDAD_CODECS: [&str; 1] = ["H264"];

//Recibe una lista de los codec en comun entre dos clientes y devuelve aquellos soportados por nuestra implementacion
//respetando un orden de prioridad, o un error en caso de no encontrar ninguno
pub fn elegir_codec(codecs: &[String]) -> Result<String, IceErrors> {
    for &p in &PRIORIDAD_CODECS {
        if codecs.iter().any(|c| c == p) {
            return Ok(p.to_string());
        }
    }
    Err(IceErrors::NoValidCodec)
}

pub fn crear_conexion_ice(
    sockets: Vec<UdpSocket>,
    peer_candidates: &[String],
) -> Result<UdpSocket, IceErrors> {
    println!(
        "[ICE] Trying to connect with {} peer candidates",
        peer_candidates.len()
    );
    log_c!(
        "[ICE] Trying to connect with {} peer candidates",
        peer_candidates.len()
    );

    // obtengo las direcciones de los candidatos del otro
    // formato ejemplo: 1 1 UDP 2130706431 192.168.1.100 5000 typ host
    let peer_addresses: Vec<String> = peer_candidates
        .iter()
        .filter_map(|candidate| {
            let parts: Vec<&str> = candidate.split_whitespace().collect();
            if parts.len() >= 6 {
                let address = format!("{}:{}", parts[4], parts[5]);
                println!("[ICE] Peer candidate: {}", address);
                log_c!("[ICE] Peer candidate: {}", address);
                Some(address)
            } else {
                None
            }
        })
        .collect();

    if peer_addresses.is_empty() {
        return Err(IceErrors::FailedConnection);
    }

    // itero los sockets y veo si alguno conecta
    for socket in sockets {
        for peer_addr in &peer_addresses {
            println!("[ICE] Trying connection to {}", peer_addr);
            log_c!("[ICE] Trying connection to {}", peer_addr);
            match socket.connect(peer_addr) {
                Ok(_) => {
                    // Testeo parecido a stun, mando algo y veo si funciona
                    let test_data = [0x00u8; 4];
                    match socket.send(&test_data) {
                        Ok(_) => {
                            println!("[ICE] Successful connection to {}", peer_addr);
                            log_c!("[ICE] Successful connection to {}", peer_addr);
                            return Ok(socket);
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
    println!("[ICE] All connection attempts failed");
    log_c!("[ICE] All connection attempts failed");
    Err(IceErrors::FailedConnection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devuelve_codec_valido_de_mayor_prioridad() {
        let codecs = vec!["VP8".to_string(), "H264".to_string()];
        let resultado = elegir_codec(&codecs).unwrap();
        assert_eq!(resultado, "H264");
    }

    #[test]
    fn test_no_codec_valido() {
        let codecs = vec!["AV1".to_string(), "Opus".to_string()];
        let resultado = elegir_codec(&codecs);

        assert!(resultado.is_err());
        assert!(matches!(resultado, Err(IceErrors::NoValidCodec)));
    }
}
