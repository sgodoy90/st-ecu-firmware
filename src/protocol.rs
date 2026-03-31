use crate::board::{board_definition, PinCapability};
use crate::config::{ConfigPageStatus, PAGE_DIRECTORY};
use crate::contract::{base_capabilities, Capability, FirmwareIdentity, TABLE_DIRECTORY};
use crate::io::{EcuFunction, ResolvedPinAssignment};
use crate::network::{MessageClass, NetworkProfile, ProductTrack, TransportLinkKind};
use crate::pinmux::PinFunctionClass;
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
    GetPinDirectory = 0x28,
    PinDirectory = 0x29,
    GetPinAssignments = 0x2A,
    PinAssignments = 0x2B,
    GetPageStatuses = 0x2C,
    PageStatuses = 0x2D,
    GetNetworkProfile = 0x2E,
    NetworkProfile = 0x2F,
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
            0x28 => Ok(Self::GetPinDirectory),
            0x29 => Ok(Self::PinDirectory),
            0x2A => Ok(Self::GetPinAssignments),
            0x2B => Ok(Self::PinAssignments),
            0x2C => Ok(Self::GetPageStatuses),
            0x2D => Ok(Self::PageStatuses),
            0x2E => Ok(Self::GetNetworkProfile),
            0x2F => Ok(Self::NetworkProfile),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPinDirectoryEntry {
    pub pin_id: String,
    pub label: String,
    pub electrical_class: String,
    pub flags: u16,
    pub timer_instance: String,
    pub timer_channel: String,
    pub adc_instance: String,
    pub adc_channel: Option<u8>,
    pub board_path: String,
    pub routes: Vec<DecodedPinRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPinRoute {
    pub function_class: PinFunctionClass,
    pub mux_mode: String,
    pub signal: String,
    pub exclusive_resource: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPinAssignment {
    pub function: EcuFunction,
    pub pin_id: String,
    pub pin_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPageStatus {
    pub page_id: u8,
    pub ram_crc: u32,
    pub flash_crc: u32,
    pub needs_burn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedNetworkLink {
    pub kind: TransportLinkKind,
    pub realtime_safe: bool,
    pub firmware_update_allowed: bool,
    pub classes: Vec<MessageClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedNetworkProfile {
    pub product_track: ProductTrack,
    pub multi_master_can: bool,
    pub links: Vec<DecodedNetworkLink>,
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

pub fn encode_pin_directory_payload() -> Vec<u8> {
    let pins = board_definition().pins;
    let mut out = Vec::new();
    out.push(pins.len() as u8);
    for pin in pins {
        encode_pin_capability(&mut out, pin);
    }
    out
}

pub fn decode_pin_directory_payload(
    payload: &[u8],
) -> Result<Vec<DecodedPinDirectoryEntry>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let pin_id = read_string(payload, &mut offset)?;
        let label = read_string(payload, &mut offset)?;
        let electrical_class = read_string(payload, &mut offset)?;
        if payload.len() < offset + 2 {
            return Err(ProtocolError::MalformedPayload);
        }
        let flags = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        offset += 2;
        let timer_instance = read_string(payload, &mut offset)?;
        let timer_channel = read_string(payload, &mut offset)?;
        let adc_instance = read_string(payload, &mut offset)?;
        let adc_channel_raw = *payload.get(offset).ok_or(ProtocolError::MalformedPayload)?;
        offset += 1;
        let board_path = read_string(payload, &mut offset)?;
        let route_count = *payload.get(offset).ok_or(ProtocolError::MalformedPayload)? as usize;
        offset += 1;
        let mut routes = Vec::with_capacity(route_count);
        for _ in 0..route_count {
            let function_class = PinFunctionClass::try_from(
                *payload.get(offset).ok_or(ProtocolError::MalformedPayload)?,
            )
            .map_err(|_| ProtocolError::MalformedPayload)?;
            offset += 1;
            let mux_mode = read_string(payload, &mut offset)?;
            let signal = read_string(payload, &mut offset)?;
            let exclusive_resource = read_string(payload, &mut offset)?;
            routes.push(DecodedPinRoute {
                function_class,
                mux_mode,
                signal,
                exclusive_resource: (!exclusive_resource.is_empty()).then_some(exclusive_resource),
            });
        }
        entries.push(DecodedPinDirectoryEntry {
            pin_id,
            label,
            electrical_class,
            flags,
            timer_instance,
            timer_channel,
            adc_instance,
            adc_channel: (adc_channel_raw != u8::MAX).then_some(adc_channel_raw),
            board_path,
            routes,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
}

pub fn encode_pin_assignments_payload(assignments: &[ResolvedPinAssignment]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(assignments.len() as u8);
    for assignment in assignments {
        out.push(assignment.function.code());
        push_string(&mut out, assignment.pin_id);
        push_string(&mut out, assignment.pin_label);
    }
    out
}

pub fn decode_pin_assignments_payload(
    payload: &[u8],
) -> Result<Vec<DecodedPinAssignment>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let mut offset = 1usize;
    let mut assignments = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let function_code = *payload.get(offset).ok_or(ProtocolError::MalformedPayload)?;
        offset += 1;
        let function =
            EcuFunction::try_from(function_code).map_err(|_| ProtocolError::MalformedPayload)?;
        let pin_id = read_string(payload, &mut offset)?;
        let pin_label = read_string(payload, &mut offset)?;
        assignments.push(DecodedPinAssignment {
            function,
            pin_id,
            pin_label,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(assignments)
}

pub fn encode_page_statuses_payload(statuses: &[ConfigPageStatus]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + statuses.len() * 10);
    out.push(statuses.len() as u8);
    for status in statuses {
        out.push(status.page_id);
        out.extend_from_slice(&status.ram_crc.to_be_bytes());
        out.extend_from_slice(&status.flash_crc.to_be_bytes());
        out.push(status.needs_burn as u8);
    }
    out
}

pub fn decode_page_statuses_payload(
    payload: &[u8],
) -> Result<Vec<DecodedPageStatus>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let expected_len = 1 + count as usize * 10;
    if payload.len() != expected_len {
        return Err(ProtocolError::MalformedPayload);
    }

    let mut offset = 1usize;
    let mut statuses = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let page_id = payload[offset];
        let ram_crc = u32::from_be_bytes([
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
            payload[offset + 4],
        ]);
        let flash_crc = u32::from_be_bytes([
            payload[offset + 5],
            payload[offset + 6],
            payload[offset + 7],
            payload[offset + 8],
        ]);
        let needs_burn = payload[offset + 9] != 0;
        statuses.push(DecodedPageStatus {
            page_id,
            ram_crc,
            flash_crc,
            needs_burn,
        });
        offset += 10;
    }

    Ok(statuses)
}

pub fn encode_network_profile_payload(profile: &NetworkProfile) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(profile.product_track.code());
    out.push(profile.multi_master_can as u8);
    out.push(profile.links.len() as u8);
    for link in profile.links {
        out.push(link.kind.code());
        out.push(link.realtime_safe as u8);
        out.push(link.firmware_update_allowed as u8);
        out.push(link.classes.len() as u8);
        for class in link.classes {
            out.push(class.code());
        }
    }
    out
}

pub fn decode_network_profile_payload(
    payload: &[u8],
) -> Result<DecodedNetworkProfile, ProtocolError> {
    if payload.len() < 3 {
        return Err(ProtocolError::MalformedPayload);
    }

    let product_track =
        ProductTrack::try_from(payload[0]).map_err(|_| ProtocolError::MalformedPayload)?;
    let multi_master_can = payload[1] != 0;
    let link_count = payload[2] as usize;
    let mut offset = 3usize;
    let mut links = Vec::with_capacity(link_count);

    for _ in 0..link_count {
        if payload.len() < offset + 4 {
            return Err(ProtocolError::MalformedPayload);
        }
        let kind = TransportLinkKind::try_from(payload[offset])
            .map_err(|_| ProtocolError::MalformedPayload)?;
        let realtime_safe = payload[offset + 1] != 0;
        let firmware_update_allowed = payload[offset + 2] != 0;
        let class_count = payload[offset + 3] as usize;
        offset += 4;
        if payload.len() < offset + class_count {
            return Err(ProtocolError::MalformedPayload);
        }
        let mut classes = Vec::with_capacity(class_count);
        for code in &payload[offset..offset + class_count] {
            classes
                .push(MessageClass::try_from(*code).map_err(|_| ProtocolError::MalformedPayload)?);
        }
        offset += class_count;
        links.push(DecodedNetworkLink {
            kind,
            realtime_safe,
            firmware_update_allowed,
            classes,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(DecodedNetworkProfile {
        product_track,
        multi_master_can,
        links,
    })
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

fn encode_pin_capability(out: &mut Vec<u8>, pin: &PinCapability) {
    push_string(out, pin.pin_id);
    push_string(out, pin.label);
    push_string(out, pin.electrical_class.key());

    let mut flags = 0u16;
    flags |= u16::from(pin.reserved) << 0;
    flags |= u16::from(pin.supports_adc) << 1;
    flags |= u16::from(pin.supports_pwm) << 2;
    flags |= u16::from(pin.supports_capture) << 3;
    flags |= u16::from(pin.supports_gpio_in) << 4;
    flags |= u16::from(pin.supports_gpio_out) << 5;
    flags |= u16::from(pin.supports_can) << 6;
    flags |= u16::from(pin.supports_uart) << 7;
    flags |= u16::from(pin.supports_spi) << 8;
    flags |= u16::from(pin.supports_i2c) << 9;
    out.extend_from_slice(&flags.to_be_bytes());

    push_string(out, pin.timer_instance.unwrap_or(""));
    push_string(out, pin.timer_channel.unwrap_or(""));
    push_string(out, pin.adc_instance.unwrap_or(""));
    out.push(pin.adc_channel.unwrap_or(u8::MAX));
    push_string(out, pin.board_path.key());
    out.push(pin.routes.len() as u8);
    for route in pin.routes {
        out.push(route.function_class.code());
        push_string(out, route.mux_mode);
        push_string(out, route.signal);
        push_string(out, route.exclusive_resource.unwrap_or(""));
    }
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
    use crate::config::ConfigPageStatus;
    use crate::contract::{base_capabilities, Capability, FirmwareIdentity};
    use crate::io::{EcuFunction, ResolvedPinAssignment};
    use crate::network::{display_network_profile, MessageClass, ProductTrack, TransportLinkKind};
    use crate::pinmux::PinFunctionClass;

    use super::{
        decode_ack_payload, decode_capabilities_payload, decode_identity_payload,
        decode_nack_payload, decode_network_profile_payload, decode_page_payload,
        decode_page_request, decode_page_statuses_payload, decode_pin_assignments_payload,
        decode_pin_directory_payload, encode_ack_payload, encode_capabilities_payload,
        encode_identity_payload, encode_nack_payload, encode_network_profile_payload,
        encode_page_directory_payload, encode_page_payload, encode_page_request,
        encode_page_statuses_payload, encode_pin_assignments_payload, encode_pin_directory_payload,
        encode_table_directory_payload, Cmd, DecodedPinAssignment, Packet, ProtocolError,
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

    #[test]
    fn pin_directory_payload_roundtrip() {
        let payload = encode_pin_directory_payload();
        let decoded = decode_pin_directory_payload(&payload).unwrap();
        assert!(decoded.iter().any(|pin| pin.pin_id == "PA0"));
        assert!(decoded.iter().any(|pin| {
            pin.pin_id == "PC8"
                && pin.board_path == "solenoid_pwm_driver"
                && pin.routes.iter().any(|route| {
                    route.function_class == PinFunctionClass::PwmOutput
                        && route.signal == "TIM3_CH3"
                })
        }));
    }

    #[test]
    fn pin_assignments_payload_roundtrip() {
        let assignments = vec![
            ResolvedPinAssignment {
                function: EcuFunction::BoostControl,
                pin_id: "PB0",
                pin_label: "BOOST_PWM",
                required_function: PinFunctionClass::PwmOutput,
            },
            ResolvedPinAssignment {
                function: EcuFunction::MapSensor,
                pin_id: "PC0",
                pin_label: "MAP",
                required_function: PinFunctionClass::AnalogInput,
            },
        ];
        let payload = encode_pin_assignments_payload(&assignments);
        let decoded: Vec<DecodedPinAssignment> = decode_pin_assignments_payload(&payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].function, EcuFunction::BoostControl);
        assert_eq!(decoded[1].pin_id, "PC0");
    }

    #[test]
    fn page_statuses_payload_roundtrip() {
        let statuses = vec![
            ConfigPageStatus {
                page_id: 0,
                ram_crc: 11,
                flash_crc: 22,
                needs_burn: true,
                flash_generation: 1,
                flash_valid: true,
            },
            ConfigPageStatus {
                page_id: 3,
                ram_crc: 33,
                flash_crc: 33,
                needs_burn: false,
                flash_generation: 2,
                flash_valid: true,
            },
        ];

        let payload = encode_page_statuses_payload(&statuses);
        let decoded = decode_page_statuses_payload(&payload).unwrap();

        assert_eq!(decoded.len(), 2);
        assert!(decoded[0].needs_burn);
        assert_eq!(decoded[1].page_id, 3);
    }

    #[test]
    fn network_profile_payload_roundtrip() {
        let payload = encode_network_profile_payload(display_network_profile());
        let decoded = decode_network_profile_payload(&payload).unwrap();

        assert_eq!(decoded.product_track, ProductTrack::DisplayIntegratedVcu);
        assert!(decoded.multi_master_can);
        assert!(decoded
            .links
            .iter()
            .any(|link| link.kind == TransportLinkKind::LocalDisplayLink));
        assert!(decoded.links.iter().any(|link| {
            link.kind == TransportLinkKind::UsbSerial
                && link.classes.contains(&MessageClass::FirmwareUpdate)
        }));
    }
}
