// Definición de tipos de paquetes RTCP según estandar RFC 3550
pub const RTCP_SR: u8 = 200;
pub const RTCP_RR: u8 = 201;
pub const RTCP_BYE: u8 = 203;

#[derive(Debug, Clone)]
pub enum RtcpPacket {
    SenderReport(SenderReport),
    ReceiverReport(ReceiverReport),
    Bye(Bye),
}
impl RtcpPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            RtcpPacket::SenderReport(sr) => sr.to_bytes(),
            RtcpPacket::ReceiverReport(rr) => rr.to_bytes(),
            RtcpPacket::Bye(bye) => bye.to_bytes(),
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let packet_type = data[1];

        match packet_type {
            RTCP_SR => SenderReport::from_bytes(data).map(RtcpPacket::SenderReport),
            RTCP_RR => ReceiverReport::from_bytes(data).map(RtcpPacket::ReceiverReport),
            RTCP_BYE => Bye::from_bytes(data).map(RtcpPacket::Bye),
            _ => None, // existen otros tipos  por simplicidad, los ignoramos
        }
    }
}
///estructura para almacenar los datos para estadisticas, tiempo de delay, bytes perdidos
#[derive(Debug, Clone, Default)]
pub struct ReportBlock {
    pub ssrc: u32,
    pub fraction_lost: u8,
    pub cumulative_lost: u32,
    pub ext_seq_num: u32,
    pub jitter: u32,
    pub last_sr_timestamp: u32,
    pub delay_since_last_sr: u32,
}

impl ReportBlock {
    fn to_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.ssrc.to_be_bytes());
        let lost_pack = ((self.fraction_lost as u32) << 24) | (self.cumulative_lost & 0xFFFFFF);
        out.extend_from_slice(&lost_pack.to_be_bytes());
        out.extend_from_slice(&self.ext_seq_num.to_be_bytes());
        out.extend_from_slice(&self.jitter.to_be_bytes());
        out.extend_from_slice(&self.last_sr_timestamp.to_be_bytes());
        out.extend_from_slice(&self.delay_since_last_sr.to_be_bytes());
    }

    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 24 {
            return None;
        }
        let ssrc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let lost_pack = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let fraction_lost = (lost_pack >> 24) as u8;
        let cumulative_lost = lost_pack & 0xFFFFFF;
        let ext_seq_num = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let jitter = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let last_sr_timestamp = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let delay_since_last_sr = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

        Some(Self {
            ssrc,
            fraction_lost,
            cumulative_lost,
            ext_seq_num,
            jitter,
            last_sr_timestamp,
            delay_since_last_sr,
        })
    }
}
///Para cada uno de los reportes pedidos: Sender, Reciever y Bye se sigue el mismo camino
/// Primero se define sus valores, luego su funcion de serializacion (to bytes) y
/// finalmente su funcion de deserializacion (from bytes) para reconstruir la estructura

#[derive(Debug, Clone)]
pub struct SenderReport {
    pub ssrc: u32,
    pub ntp_timestamp: u64,
    pub rtp_timestamp: u32,
    pub packet_count: u32,
    pub octet_count: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl SenderReport {
    pub fn new(ssrc: u32, ntp: u64, rtp_ts: u32, pkts: u32, octets: u32) -> Self {
        Self {
            ssrc,
            ntp_timestamp: ntp,
            rtp_timestamp: rtp_ts,
            packet_count: pkts,
            octet_count: octets,
            report_blocks: Vec::new(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let rc = (self.report_blocks.len() as u8) & 0x1F;
        let v_p_rc = (2 << 6) | rc;
        out.push(v_p_rc);
        out.push(RTCP_SR);
        let length_words = 6 + (self.report_blocks.len() as u16 * 6);
        out.extend_from_slice(&(length_words - 1).to_be_bytes());

        out.extend_from_slice(&self.ssrc.to_be_bytes());
        out.extend_from_slice(&self.ntp_timestamp.to_be_bytes());
        out.extend_from_slice(&self.rtp_timestamp.to_be_bytes());
        out.extend_from_slice(&self.packet_count.to_be_bytes());
        out.extend_from_slice(&self.octet_count.to_be_bytes());

        for rb in &self.report_blocks {
            rb.to_bytes(&mut out);
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }
        let rc = data[0] & 0x1F;
        let ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ntp_timestamp = u64::from_be_bytes(data[8..16].try_into().ok()?);
        let rtp_timestamp = u32::from_be_bytes(data[16..20].try_into().ok()?);
        let packet_count = u32::from_be_bytes(data[20..24].try_into().ok()?);
        let octet_count = u32::from_be_bytes(data[24..28].try_into().ok()?);

        let mut report_blocks = Vec::new();
        let mut offset = 28;
        for _ in 0..rc {
            if offset + 24 > data.len() {
                break;
            }
            if let Some(rb) = ReportBlock::from_bytes(&data[offset..offset + 24]) {
                report_blocks.push(rb);
            }
            offset += 24;
        }

        Some(Self {
            ssrc,
            ntp_timestamp,
            rtp_timestamp,
            packet_count,
            octet_count,
            report_blocks,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReceiverReport {
    pub ssrc: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl ReceiverReport {
    pub fn new(ssrc: u32, report_blocks: Vec<ReportBlock>) -> Self {
        Self {
            ssrc,
            report_blocks,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let rc = (self.report_blocks.len() as u8) & 0x1F;
        let v_p_rc = (2 << 6) | rc;
        out.push(v_p_rc);
        out.push(RTCP_RR);

        let length_words = 1 + (self.report_blocks.len() as u16 * 6);
        out.extend_from_slice(&(length_words - 1).to_be_bytes());
        out.extend_from_slice(&self.ssrc.to_be_bytes());

        for rb in &self.report_blocks {
            rb.to_bytes(&mut out);
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        let rc = data[0] & 0x1F;
        let ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        let mut report_blocks = Vec::new();
        let mut offset = 8;
        for _ in 0..rc {
            if offset + 24 > data.len() {
                break;
            }
            if let Some(rb) = ReportBlock::from_bytes(&data[offset..offset + 24]) {
                report_blocks.push(rb);
            }
            offset += 24;
        }
        Some(Self {
            ssrc,
            report_blocks,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Bye {
    pub ssrcs: Vec<u32>,
    pub reason: Option<String>,
}

impl Bye {
    pub fn new(ssrc: u32, reason: Option<String>) -> Self {
        Self {
            ssrcs: vec![ssrc],
            reason,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let sc = (self.ssrcs.len() as u8) & 0x1F;
        let v_p_sc = (2 << 6) | sc;
        out.push(v_p_sc);
        out.push(RTCP_BYE);

        let mut length_words = self.ssrcs.len() as u16;
        let mut reason_bytes = Vec::new();

        if let Some(r) = &self.reason {
            let msg = r.as_bytes();
            let len = msg.len().min(255) as u8;
            reason_bytes.push(len);
            reason_bytes.extend_from_slice(&msg[..len as usize]);
            while reason_bytes.len() % 4 != 0 {
                reason_bytes.push(0);
            }
            length_words += (reason_bytes.len() / 4) as u16;
        }

        out.extend_from_slice(&(length_words).to_be_bytes());

        for ssrc in &self.ssrcs {
            out.extend_from_slice(&ssrc.to_be_bytes());
        }
        out.extend_from_slice(&reason_bytes);
        out
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let sc = (data[0] & 0x1F) as usize;
        let mut offset = 4;
        let mut ssrcs = Vec::new();
        for _ in 0..sc {
            if offset + 4 > data.len() {
                break;
            }
            ssrcs.push(u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]));
            offset += 4;
        }

        let mut reason = None;
        if offset < data.len() {
            let len = data[offset] as usize;
            if offset + 1 + len <= data.len()
                && let Ok(r) = String::from_utf8(data[offset + 1..offset + 1 + len].to_vec())
            {
                reason = Some(r);
            }
        }
        Some(Self { ssrcs, reason })
    }
}
