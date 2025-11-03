use crate::ice_errors::IceErrors;
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

pub fn crear_conexion_ice(my_ip: &[String], peer_ip: &[String]) -> Result<UdpSocket, IceErrors> {
    //     let socket = UdpSocket::bind(my_addres).map_err(ClientErrors::CantBind)?;
    //    socket.connect(peer).map_err(ClientErrors::CantConect)?;

    // Itero los IP:puertos hasta lograr una conexion
    for my_addr in my_ip {
        for peer_addr in peer_ip {
            //PRUEBO CONECTAR, SI FALLA ITERO HASTA QUE NO UNO FALLE
            match UdpSocket::bind(my_addr) {
                Ok(socket) => match socket.connect(peer_addr) {
                    Ok(_) => return Ok(socket),
                    Err(_) => continue,
                },
                Err(_) => continue,
            }
        }
    }
    // Si falla, devolver un error
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
