use crate::config::PAGE_DIRECTORY;
use crate::contract::{base_capabilities, Capability, FirmwareIdentity, TABLE_DIRECTORY};
use crc::{Crc, CRC_16_IBM_SDLC, CRC_32_ISO_HDLC};

static CRC16: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_SDLC);
static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub const MAGIC: [u8; 2] = [0x53, 0x54];
pub const MAX_PAYLOAD: usize = 8192;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Ping = 0x01,
    Pong = 0x02,
    GetVersion = 0x03,
    VersionResponse = 0x04,
    GetCapabilities = 0x05,
    Capabilities = 0x06,
    GetLiveData = 0x07,
    LiveData = 0x08,
    ReadPage = 0x20,
    PageData = 0x21,
    WritePage = 0x22,
    BurnPage = 0x23,
    GetPageDirectory = 0x24,
    PageDirectory = 0x25,
    GetTableDirectory = 0x26,
    TableDirectory = 0x27,
    Ack = 0xA0,
    Nack = 0xA1,
    Error = 0xFF,
}

impl TryFrom<u8> for Cmd {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0x01 => Ok(Self::Ping),
            0x02 => Ok(Self::Pong),
            0x03 => Ok(Self::GetVersion),
            0x04 => Ok(Self::VersionResponse),
            0x05 => Ok(Self::GetCapabilities),
            0x06 => Ok(Self::Capabilities),
            0x07 => Ok(Self::GetLiveData),
            0x08 => Ok(Self::LiveData),
            0x20 => Ok(Self::ReadPage),
            0x21 => Ok(Self::PageData),
            0x22 => Ok(Self::WritePage),
            0x23 => Ok(Self::BurnPage),
            0x24 => Ok(Self::GetPageDirectory),
            0x25 => Ok(Self::PageDirectory),
            0x26 => Ok(Self::GetTableDirectory),
            0x27 => Ok(Self::TableDirectory),
            0xA0 => Ok(Self::Ack),
            0xA1 => Ok(Self::Nack),
            0xFF => Ok(Self::Error),
            _ => Err(ProtocolError::UnknownCmd(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub cmd: Cmd,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn new(cmd: Cmd, payload: Vec<u8>) -> Self {
        Self { cmd, payload }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let len = self.payload.len();
        let mut out = Vec::with_capacity(7 + len);
        out.extend_from_slice(&MAGIC);
        out.push(((len >> 8) & 0xFF) as u8);
        out.push((len & 0xFF) as u8);
        out.push(self.cmd as u8);
        out.extend_from_slice(&self.payload);
        let crc = CRC16.checksum(&out[2..]);
        out.push((crc >> 8) as u8);
        out.push((crc & 0xFF) as u8);
        out
    }

    pub fn from_bytes(data: &[u8]) -> Result<Option<(Self, usize)>, ProtocolError> {
        if data.len() < 7 {
            return Ok(None);
        }
        if data[0] != MAGIC[0] || data[1] != MAGIC[1] {
            return Err(ProtocolError::BadMagic);
        }
        let len = ((data[2] as usize) << 8) | data[3] as usize;
        if len > MAX_PAYLOAD {
            return Err(ProtocolError::TooLarge(len));
        }
        let total = 5 + len + 2;
        if data.len() < total {
            return Ok(None);
        }
        let expected = CRC16.checksum(&data[2..total - 2]);
        let actual = ((data[total - 2] as u16) << 8) | data[total - 1] as u16;
        if expected != actual {
            return Err(ProtocolError::CrcFail { expected, actual });
        }
        Ok(Some((
            Self {
                cmd: Cmd::try_from(data[4])?,
                payload: data[5..total - 2].to_vec(),
            },
            total,
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    BadMagic,
    TooLarge(usize),
    CrcFail { expected: u16, actual: u16 },
    UnknownCmd(u8),
    MalformedPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedIdentity {
    pub protocol_version: u8,
    pub schema_version: u16,
    pub firmware_id: String,
    pub firmware_semver: String,
    pub board_id: String,
    pub serial: String,
    pub signature: String,
    pub capabilities: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPagePayload {
    pub page_id: u8,
    pub payload_crc: u32,
    pub payload: Vec<u8>,
}

pub fn encode_identity_payload(
    identity: &FirmwareIdentity,
    capabilities: &[Capability],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(identity.protocol_version);
    out.extend_from_slice(&identity.schema_version.to_be_bytes());
    out.push(capabilities.len() as u8);
    push_string(&mut out, identity.firmware_id);
    push_string(&mut out, identity.firmware_semver);
    push_string(&mut out, identity.board_id);
    push_string(&mut out, identity.serial);
    push_string(&mut out, identity.signature);
    for capability in capabilities {
        out.push(capability.code());
    }
    out
}

pub fn decode_identity_payload(payload: &[u8]) -> Result<DecodedIdentity, ProtocolError> {
    if payload.len() < 4 {
        return Err(ProtocolError::MalformedPayload);
    }
    let mut offset = 0usize;
    let protocol_version = payload[offset];
    offset += 1;
    let schema_version = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let capability_count = payload[offset] as usize;
    offset += 1;

    let firmware_id = read_string(payload, &mut offset)?;
    let firmware_semver = read_string(payload, &mut offset)?;
    let board_id = read_string(payload, &mut offset)?;
    let serial = read_string(payload, &mut offset)?;
    let signature = read_string(payload, &mut offset)?;

    if payload.len() < offset + capability_count {
        return Err(ProtocolError::MalformedPayload);
    }
    let capabilities = payload[offset..offset + capability_count].to_vec();

    Ok(DecodedIdentity {
        protocol_version,
        schema_version,
        firmware_id,
        firmware_semver,
        board_id,
        serial,
        signature,
        capabilities,
    })
}

pub fn encode_page_directory_payload() -> Vec<u8> {
    let mut out = Vec::new();
    out.push(PAGE_DIRECTORY.len() as u8);
    for page in PAGE_DIRECTORY {
        out.push(page.id);
        out.extend_from_slice(&page.byte_length.to_be_bytes());
        push_string(&mut out, page.key);
    }
    out
}

pub fn encode_capabilities_payload(capabilities: &[Capability]) -> Vec<u8> {
    let mut out = Vec::with_capacity(capabilities.len() + 1);
    out.push(capabilities.len() as u8);
    for capability in capabilities {
        out.push(capability.code());
    }
    out
}

pub fn decode_capabilities_payload(payload: &[u8]) -> Result<Vec<Capability>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };
    if payload.len() != count as usize + 1 {
        return Err(ProtocolError::MalformedPayload);
    }

    payload[1..]
        .iter()
        .copied()
        .map(|code| Capability::try_from(code).map_err(|_| ProtocolError::MalformedPayload))
        .collect()
}

pub fn encode_table_directory_payload() -> Vec<u8> {
    let mut out = Vec::new();
    out.push(TABLE_DIRECTORY.len() as u8);
    for table in TABLE_DIRECTORY {
        out.push(table.id);
        out.push(table.x_count);
        out.push(table.y_count);
        out.push(table.signed as u8);
        push_string(&mut out, table.key);
    }
    out
}

pub fn encode_page_request(page_id: u8) -> Vec<u8> {
    vec![page_id]
}

pub fn decode_page_request(payload: &[u8]) -> Result<u8, ProtocolError> {
    if payload.len() != 1 {
        return Err(ProtocolError::MalformedPayload);
    }
    Ok(payload[0])
}

pub fn encode_page_payload(page_id: u8, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 7);
    out.push(page_id);
    out.extend_from_slice(&(data.len() as u16).to_be_bytes());
    out.extend_from_slice(&CRC32.checksum(data).to_be_bytes());
    out.extend_from_slice(data);
    out
}

pub fn encode_ack_payload(page_id: u8, needs_burn: bool) -> Vec<u8> {
    vec![page_id, needs_burn as u8]
}

pub fn decode_ack_payload(payload: &[u8]) -> Result<(u8, bool), ProtocolError> {
    if payload.len() != 2 {
        return Err(ProtocolError::MalformedPayload);
    }
    Ok((payload[0], payload[1] != 0))
}

pub fn encode_nack_payload(code: u8, reason: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(reason.len() + 2);
    out.push(code);
    push_string(&mut out, reason);
    out
}

pub fn decode_nack_payload(payload: &[u8]) -> Result<(u8, String), ProtocolError> {
    if payload.is_empty() {
        return Err(ProtocolError::MalformedPayload);
    }
    let mut offset = 1usize;
    let reason = read_string(payload, &mut offset)?;
    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }
    Ok((payload[0], reason))
}

pub fn decode_page_payload(payload: &[u8]) -> Result<DecodedPagePayload, ProtocolError> {
    if payload.len() < 7 {
        return Err(ProtocolError::MalformedPayload);
    }

    let page_id = payload[0];
    let len = u16::from_be_bytes([payload[1], payload[2]]) as usize;
    if payload.len() != len + 7 {
        return Err(ProtocolError::MalformedPayload);
    }

    let payload_crc = u32::from_be_bytes([payload[3], payload[4], payload[5], payload[6]]);
    let data = payload[7..].to_vec();
    let actual_crc = CRC32.checksum(&data);
    if payload_crc != actual_crc {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(DecodedPagePayload {
        page_id,
        payload_crc,
        payload: data,
    })
}

pub fn simulator_identity_payload() -> Vec<u8> {
    encode_identity_payload(&FirmwareIdentity::simulator(), &base_capabilities(true))
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.push(value.len() as u8);
    out.extend_from_slice(value.as_bytes());
}

fn read_string(payload: &[u8], offset: &mut usize) -> Result<String, ProtocolError> {
    let len = *payload
        .get(*offset)
        .ok_or(ProtocolError::MalformedPayload)? as usize;
    *offset += 1;
    let end = *offset + len;
    if payload.len() < end {
        return Err(ProtocolError::MalformedPayload);
    }
    let value = String::from_utf8(payload[*offset..end].to_vec())
        .map_err(|_| ProtocolError::MalformedPayload)?;
    *offset = end;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::contract::{base_capabilities, Capability, FirmwareIdentity};

    use super::{
        decode_ack_payload, decode_capabilities_payload, decode_identity_payload,
        decode_nack_payload, decode_page_payload, decode_page_request, encode_ack_payload,
        encode_capabilities_payload, encode_identity_payload, encode_nack_payload,
        encode_page_directory_payload, encode_page_payload, encode_page_request,
        encode_table_directory_payload, Cmd, Packet, ProtocolError,
    };

    #[test]
    fn packet_roundtrip() {
        let packet = Packet::new(Cmd::Ping, vec![1, 2, 3, 4]);
        let bytes = packet.to_bytes();
        let parsed = Packet::from_bytes(&bytes).unwrap().unwrap();
        assert_eq!(parsed.0, packet);
    }

    #[test]
    fn identity_payload_roundtrip() {
        let payload =
            encode_identity_payload(&FirmwareIdentity::ecu_v1(), &base_capabilities(false));
        let decoded = decode_identity_payload(&payload).unwrap();
        assert_eq!(decoded.protocol_version, 1);
        assert_eq!(decoded.schema_version, 1);
        assert_eq!(decoded.board_id, "st-ecu-v1");
        assert!(!decoded.capabilities.is_empty());
    }

    #[test]
    fn capabilities_payload_roundtrip() {
        let payload = encode_capabilities_payload(&base_capabilities(true));
        let decoded = decode_capabilities_payload(&payload).unwrap();
        assert!(decoded.contains(&Capability::LiveData));
        assert!(decoded.contains(&Capability::Simulator));
    }

    #[test]
    fn directories_encode_entries() {
        let pages = encode_page_directory_payload();
        let tables = encode_table_directory_payload();
        assert!(pages.len() > 4);
        assert!(tables.len() > 4);
    }

    #[test]
    fn bad_magic_fails() {
        let packet = Packet::new(Cmd::Ping, vec![]);
        let mut bytes = packet.to_bytes();
        bytes[0] = 0;
        let parsed = Packet::from_bytes(&bytes);
        assert!(matches!(parsed, Err(ProtocolError::BadMagic)));
    }

    #[test]
    fn page_request_roundtrip() {
        let payload = encode_page_request(3);
        assert_eq!(decode_page_request(&payload).unwrap(), 3);
    }

    #[test]
    fn page_payload_roundtrip() {
        let payload = encode_page_payload(2, &[1, 2, 3, 4, 5]);
        let decoded = decode_page_payload(&payload).unwrap();
        assert_eq!(decoded.page_id, 2);
        assert_eq!(decoded.payload, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn malformed_page_payload_fails() {
        let mut payload = encode_page_payload(1, &[9, 8, 7]);
        *payload.last_mut().unwrap() ^= 0xFF;
        assert!(matches!(
            decode_page_payload(&payload),
            Err(ProtocolError::MalformedPayload)
        ));
    }

    #[test]
    fn ack_payload_roundtrip() {
        let payload = encode_ack_payload(4, true);
        assert_eq!(decode_ack_payload(&payload).unwrap(), (4, true));
    }

    #[test]
    fn nack_payload_roundtrip() {
        let payload = encode_nack_payload(2, "bad page");
        assert_eq!(
            decode_nack_payload(&payload).unwrap(),
            (2, "bad page".to_string())
        );
    }
}
