use std::net::{UdpSocket, Ipv4Addr, ToSocketAddrs};
use std::time::Duration;
use rand::random;

use crate::ice_errors::IceErrors;

const STUN_MAGIC_COOKIE: u32 = 0x2112A442;



pub fn get_stun_reflexive(socket: UdpSocket) -> Result<(String, u16), IceErrors>{
    eprintln!("[STUN] Consultando STUN server");

    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    //para no tener que agregar mas 
    let mut addrs = "stun.l.google.com:19302".to_socket_addrs()?;
    let ipv4_addr = addrs
        .find(|addr| addr.is_ipv4())
        .expect("[STUN] No IPv4 STUN address found");

    socket.connect(ipv4_addr)?;
    // binding request 
    let mut req = Vec::with_capacity(20);
    req.extend_from_slice(&0x0001u16.to_be_bytes());
    req.extend_from_slice(&0u16.to_be_bytes()); // length 0
    req.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());

    // transaction id: entiendo que si se mandan mas de un request
    //en la misma red, podria confundirse la rta, uso esto para asegurarme
    let tx_id: [u8; 12] = random();
    req.extend_from_slice(&tx_id);

    socket.send(&req)?;

    //Recibo respuesta
    let mut buf = [0u8; 256];
    let n = socket.recv(&mut buf)?;

    // tengo q tener min 20 bytes
    if n < 20 {
        return Err(IceErrors::StunError);
    }

    // TBinding Response
    if &buf[0..2] != [0x01, 0x01] {
        return Err(IceErrors::StunError);
    }

    // magic cookie
    if buf[4..8] != STUN_MAGIC_COOKIE.to_be_bytes() {
        return Err(IceErrors::StunError);
    }

    // tiene que coincidir el id
    if &buf[8..20] != tx_id {
        return Err(IceErrors::StunError);
    }

    // XOR MAPPED ADDRESS 
    let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    let mut offset = 20;
    let end = 20 + msg_len;

    while offset + 4 <= end {
        let atype = u16::from_be_bytes([buf[offset], buf[offset+1]]);
        let alen  = u16::from_be_bytes([buf[offset+2], buf[offset+3]]) as usize;
        offset += 4;

        if offset + alen > end { break; }

        if atype == 0x0020 && alen >= 8 {
            return parse_xor_address(&buf[offset..offset+alen]);
        }

        // padding 32bit
        offset += (alen + 3) & !3;
    }

    Err(IceErrors::StunError)
}


fn parse_xor_address(data: &[u8]) -> Result<(String, u16), IceErrors> {
    if data.len() < 8 || data[1] != 0x01 {
        return Err(IceErrors::StunError);
    }

    let port_raw = u16::from_be_bytes([data[2], data[3]]);
    let port = port_raw ^ ((STUN_MAGIC_COOKIE >> 16) as u16);

    let ip_raw = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let ip_val = ip_raw ^ STUN_MAGIC_COOKIE;

    let ip = Ipv4Addr::from(ip_val).to_string();

    eprintln!("[STUN] Reflexive: {}:{}", ip, port);

    Ok((ip, port))
}


// LLevar a forma RFC 5245 para candidatos
pub fn get_stun_candidate(socket: &UdpSocket) -> Result<String, IceErrors> {
    let cloned_socket = socket.try_clone().map_err(|_| IceErrors::StunError)?;
    let (public_ip, public_port) = get_stun_reflexive(cloned_socket)?;
    
    let candidate = format!(
        "1 1 UDP 2130706431 {} {} typ srflx",
        public_ip, public_port
    );
    
    Ok(candidate)
}

///test

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_stun() {

        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        match get_stun_reflexive(socket) {
            Ok((public_ip, public_port)) => {
                println!("Mi STUN reflexive: {}:{}", public_ip, public_port);
            }
            Err(e) => {
                println!("STUN falló: {:?}", e);
            }
        }
    }
}