use crate::board::{board_definition, PinCapability};
use crate::config::{ConfigPageStatus, PAGE_DIRECTORY};
use crate::contract::{base_capabilities, Capability, FirmwareIdentity, TABLE_DIRECTORY};
use crate::diagnostics::FreezeFrame;
use crate::io::{EcuFunction, ResolvedPinAssignment};
use crate::network::{MessageClass, NetworkProfile, ProductTrack, TransportLinkKind};
use crate::pinmux::PinFunctionClass;
use crate::trigger::{TriggerCapture, TriggerDecoderPreset, TriggerToothLog};
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
    GetTableMetadata = 0x2A,
    TableMetadata = 0x2B,
    GetPageStatuses = 0x2C,
    PageStatuses = 0x2D,
    GetNetworkProfile = 0x2E,
    NetworkProfile = 0x2F,
    ReadTable = 0x30,
    TableData = 0x31,
    WriteTable = 0x32,
    WriteCell = 0x33,
    ReadCurve = 0x34,
    CurveData = 0x35,
    WriteCurve = 0x36,
    GetDtc = 0x40,
    DtcList = 0x41,
    ClearDtc = 0x42,
    GetSensorRaw = 0x43,
    SensorRaw = 0x44,
    RunOutputTest = 0x45,
    GetOutputTestDirectory = 0x46,
    OutputTestDirectory = 0x47,
    GetSensorRawDirectory = 0x48,
    SensorRawDirectory = 0x49,
    GetFreezeFrames = 0x4A,
    FreezeFrames = 0x4B,
    GetTriggerCapture = 0x4C,
    TriggerCapture = 0x4D,
    GetTriggerDecoderDirectory = 0x4E,
    TriggerDecoderDirectory = 0x4F,
    EnterBootloader = 0x50,
    FlashBlock = 0x51,
    FlashBlockAck = 0x52,
    FlashVerify = 0x53,
    FlashComplete = 0x54,
    GetUpdateStatus = 0x55,
    UpdateStatus = 0x56,
    ConfirmBootHealthy = 0x57,
    FlashResume = 0x58,
    LogStart = 0x60,
    LogStop = 0x61,
    LogStatus = 0x62,
    LogStatusResponse = 0x63,
    ReadLogBlock = 0x64,
    LogBlockData = 0x65,
    GetLogbookSummary = 0x66,
    LogbookSummaryResponse = 0x67,
    ResetLogbook = 0x68,
    SyncRtc = 0x69,
    GetTriggerToothLog = 0x70,
    TriggerToothLog = 0x71,
    GetCanTemplateDirectory = 0x72,
    CanTemplateDirectory = 0x73,
    GetCanSignalDirectory = 0x74,
    CanSignalDirectory = 0x75,
    GetPinAssignments = 0x6A,
    PinAssignments = 0x6B,
    PinAssign = 0x6C,
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
            0x2A => Ok(Self::GetTableMetadata),
            0x2B => Ok(Self::TableMetadata),
            0x2C => Ok(Self::GetPageStatuses),
            0x2D => Ok(Self::PageStatuses),
            0x2E => Ok(Self::GetNetworkProfile),
            0x2F => Ok(Self::NetworkProfile),
            0x30 => Ok(Self::ReadTable),
            0x31 => Ok(Self::TableData),
            0x32 => Ok(Self::WriteTable),
            0x33 => Ok(Self::WriteCell),
            0x34 => Ok(Self::ReadCurve),
            0x35 => Ok(Self::CurveData),
            0x36 => Ok(Self::WriteCurve),
            0x40 => Ok(Self::GetDtc),
            0x41 => Ok(Self::DtcList),
            0x42 => Ok(Self::ClearDtc),
            0x43 => Ok(Self::GetSensorRaw),
            0x44 => Ok(Self::SensorRaw),
            0x45 => Ok(Self::RunOutputTest),
            0x46 => Ok(Self::GetOutputTestDirectory),
            0x47 => Ok(Self::OutputTestDirectory),
            0x48 => Ok(Self::GetSensorRawDirectory),
            0x49 => Ok(Self::SensorRawDirectory),
            0x4A => Ok(Self::GetFreezeFrames),
            0x4B => Ok(Self::FreezeFrames),
            0x4C => Ok(Self::GetTriggerCapture),
            0x4D => Ok(Self::TriggerCapture),
            0x4E => Ok(Self::GetTriggerDecoderDirectory),
            0x4F => Ok(Self::TriggerDecoderDirectory),
            0x50 => Ok(Self::EnterBootloader),
            0x51 => Ok(Self::FlashBlock),
            0x52 => Ok(Self::FlashBlockAck),
            0x53 => Ok(Self::FlashVerify),
            0x54 => Ok(Self::FlashComplete),
            0x55 => Ok(Self::GetUpdateStatus),
            0x56 => Ok(Self::UpdateStatus),
            0x57 => Ok(Self::ConfirmBootHealthy),
            0x58 => Ok(Self::FlashResume),
            0x60 => Ok(Self::LogStart),
            0x61 => Ok(Self::LogStop),
            0x62 => Ok(Self::LogStatus),
            0x63 => Ok(Self::LogStatusResponse),
            0x64 => Ok(Self::ReadLogBlock),
            0x65 => Ok(Self::LogBlockData),
            0x66 => Ok(Self::GetLogbookSummary),
            0x67 => Ok(Self::LogbookSummaryResponse),
            0x68 => Ok(Self::ResetLogbook),
            0x69 => Ok(Self::SyncRtc),
            0x70 => Ok(Self::GetTriggerToothLog),
            0x71 => Ok(Self::TriggerToothLog),
            0x72 => Ok(Self::GetCanTemplateDirectory),
            0x73 => Ok(Self::CanTemplateDirectory),
            0x74 => Ok(Self::GetCanSignalDirectory),
            0x75 => Ok(Self::CanSignalDirectory),
            0x6A => Ok(Self::GetPinAssignments),
            0x6B => Ok(Self::PinAssignments),
            0x6C => Ok(Self::PinAssign),
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
    pub reset_reason: Option<String>,
    pub capabilities: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPagePayload {
    pub page_id: u8,
    pub payload_crc: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogStatusPayload {
    pub active: bool,
    pub storage_present: bool,
    pub rtc_synced: bool,
    pub logbook_entries: u8,
    pub session_id: u32,
    pub elapsed_ms: u32,
    pub bytes_written: u32,
    pub block_count: u16,
    pub block_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogbookSummaryPayload {
    pub sessions: u32,
    pub entries: u16,
    pub total_elapsed_ms: u32,
    pub total_bytes_written: u32,
    pub last_session_id: u32,
    pub last_elapsed_ms: u32,
    pub last_bytes_written: u32,
    pub last_block_count: u16,
    pub rtc_synced: bool,
    pub last_rtc_sync_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedLogBlockPayload {
    pub block_index: u16,
    pub total_blocks: u16,
    pub payload_crc: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTablePayload {
    pub table_id: u8,
    pub x_count: u8,
    pub y_count: u8,
    pub x_bins: Vec<u16>,
    pub y_bins: Vec<u16>,
    pub data: Vec<u16>,
}

impl RawTablePayload {
    pub fn to_payload(&self) -> Vec<u8> {
        let x_count = self.x_count as usize;
        let y_count = self.y_count as usize;
        let mut out = Vec::with_capacity(3 + x_count * 2 + y_count * 2 + x_count * y_count * 2);
        out.push(self.table_id);
        out.push(self.x_count);
        out.push(self.y_count);
        for value in &self.x_bins {
            out.extend_from_slice(&value.to_be_bytes());
        }
        for value in &self.y_bins {
            out.extend_from_slice(&value.to_be_bytes());
        }
        for value in &self.data {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out
    }
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
pub struct OutputTestDirectoryEntry {
    pub channel: u8,
    pub function: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub default_pulse_ms: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedOutputTestDirectoryEntry {
    pub channel: u8,
    pub function: String,
    pub label: String,
    pub group: String,
    pub default_pulse_ms: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedSensorRaw {
    pub adc: u16,
    pub voltage: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRawDirectoryEntry {
    pub channel: u8,
    pub key: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub pin_id: &'static str,
    pub pin_label: &'static str,
    pub expected_min_mv: u16,
    pub expected_max_mv: u16,
    pub fault_low_mv: u16,
    pub fault_high_mv: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSensorRawDirectoryEntry {
    pub channel: u8,
    pub key: String,
    pub label: String,
    pub unit: String,
    pub pin_id: String,
    pub pin_label: String,
    pub expected_min_mv: u16,
    pub expected_max_mv: u16,
    pub fault_low_mv: u16,
    pub fault_high_mv: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedFreezeFrame {
    pub code: String,
    pub label: String,
    pub reason: String,
    pub rev_counter: u32,
    pub rpm: u16,
    pub map_kpa: f64,
    pub tps_pct: f64,
    pub coolant_c: f64,
    pub lambda: f64,
    pub vbatt: f64,
    pub gear: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTriggerDecoderPreset {
    pub key: String,
    pub label: String,
    pub family: String,
    pub decoder: String,
    pub pattern_kind: String,
    pub primary_input_label: String,
    pub secondary_input_label: Option<String>,
    pub primary_sensor_kind: String,
    pub secondary_sensor_kind: Option<String>,
    pub edge_policy: String,
    pub sync_strategy: String,
    pub primary_pattern_hint: String,
    pub secondary_pattern_hint: Option<String>,
    pub reference_description: String,
    pub expected_engine_cycle_deg: u16,
    pub requires_secondary: bool,
    pub supports_sequential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTriggerCapture {
    pub preset_key: String,
    pub preset_label: String,
    pub sync_state: String,
    pub trigger_rpm: u16,
    pub sync_loss_counter: u32,
    pub synced_cycles: u32,
    pub engine_cycle_deg: u16,
    pub capture_window_us: u32,
    pub sample_period_us: u16,
    pub primary_label: String,
    pub secondary_label: Option<String>,
    pub tooth_count: u16,
    pub sync_gap_tooth_count: u8,
    pub primary_edge_count: u16,
    pub secondary_edge_count: u16,
    pub primary_samples: Vec<u8>,
    pub secondary_samples: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedTriggerToothLog {
    pub preset_key: String,
    pub preset_label: String,
    pub sync_state: String,
    pub trigger_rpm: u16,
    pub engine_cycle_deg: u16,
    pub primary_label: String,
    pub secondary_label: Option<String>,
    pub reference_event_index: u16,
    pub dominant_gap_ratio: f32,
    pub tooth_intervals_us: Vec<u32>,
    pub secondary_event_indexes: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanTemplateDirectoryEntry {
    pub id: u8,
    pub key: &'static str,
    pub label: &'static str,
    pub direction: &'static str,
    pub category: &'static str,
    pub can_id: u32,
    pub extended_id: bool,
    pub dlc: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanSignalDirectoryEntry {
    pub id: u8,
    pub template_id: u8,
    pub key: &'static str,
    pub label: &'static str,
    pub maps_to: &'static str,
    pub start_bit: u8,
    pub bit_length: u8,
    pub signed: bool,
    pub little_endian: bool,
    pub scale: f32,
    pub offset: f32,
    pub min: f32,
    pub max: f32,
    pub unit: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedCanTemplateDirectoryEntry {
    pub id: u8,
    pub key: String,
    pub label: String,
    pub direction: String,
    pub category: String,
    pub can_id: u32,
    pub extended_id: bool,
    pub dlc: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedCanSignalDirectoryEntry {
    pub id: u8,
    pub template_id: u8,
    pub key: String,
    pub label: String,
    pub maps_to: String,
    pub start_bit: u8,
    pub bit_length: u8,
    pub signed: bool,
    pub little_endian: bool,
    pub scale: f32,
    pub offset: f32,
    pub min: f32,
    pub max: f32,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPageStatus {
    pub page_id: u8,
    pub ram_crc: u32,
    pub flash_crc: u32,
    pub needs_burn: bool,
    pub flash_generation: u32,
    pub flash_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareUpdateStatusPayload {
    pub state: u8,
    pub active_bank: u8,
    pub last_good_bank: u8,
    pub pending_bank: Option<u8>,
    pub boot_attempts: u8,
    pub rollback_counter: u16,
    pub candidate_size: u32,
    pub candidate_crc: u32,
    pub health_window_remaining_ms: u32,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTableMetadataEntry {
    pub id: u8,
    pub x_scale: u16,
    pub y_scale: u16,
    pub value_scale: u16,
    pub min_value_raw: i32,
    pub max_value_raw: i32,
    pub default_value_raw: i32,
    pub key: String,
    pub x_label: String,
    pub y_label: String,
    pub z_label: String,
    pub unit: String,
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
    push_string(&mut out, identity.reset_reason);
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
    let mut reset_reason = None;
    if payload.len() != offset + capability_count {
        let mut extended_offset = offset;
        let parsed_reset_reason = read_string(payload, &mut extended_offset)?;
        if payload.len() != extended_offset + capability_count {
            return Err(ProtocolError::MalformedPayload);
        }
        if !parsed_reset_reason.is_empty() {
            reset_reason = Some(parsed_reset_reason);
        }
        offset = extended_offset;
    }

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
        reset_reason,
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

pub fn encode_table_metadata_payload() -> Vec<u8> {
    let mut out = Vec::new();
    out.push(TABLE_DIRECTORY.len() as u8);
    for table in TABLE_DIRECTORY {
        out.push(table.id);
        out.extend_from_slice(&table.x_scale.to_be_bytes());
        out.extend_from_slice(&table.y_scale.to_be_bytes());
        out.extend_from_slice(&table.value_scale.to_be_bytes());
        out.extend_from_slice(&table.min_value_raw.to_be_bytes());
        out.extend_from_slice(&table.max_value_raw.to_be_bytes());
        out.extend_from_slice(&table.default_value_raw.to_be_bytes());
        push_string(&mut out, table.key);
        push_string(&mut out, table.x_label);
        push_string(&mut out, table.y_label);
        push_string(&mut out, table.z_label);
        push_string(&mut out, table.unit);
    }
    out
}

pub fn decode_table_metadata_payload(
    payload: &[u8],
) -> Result<Vec<DecodedTableMetadataEntry>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if payload.len() < offset + 19 {
            return Err(ProtocolError::MalformedPayload);
        }
        let id = payload[offset];
        offset += 1;
        let x_scale = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        offset += 2;
        let y_scale = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        offset += 2;
        let value_scale = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        offset += 2;
        let min_value_raw = i32::from_be_bytes([
            payload[offset],
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
        ]);
        offset += 4;
        let max_value_raw = i32::from_be_bytes([
            payload[offset],
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
        ]);
        offset += 4;
        let default_value_raw = i32::from_be_bytes([
            payload[offset],
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
        ]);
        offset += 4;
        let key = read_string(payload, &mut offset)?;
        let x_label = read_string(payload, &mut offset)?;
        let y_label = read_string(payload, &mut offset)?;
        let z_label = read_string(payload, &mut offset)?;
        let unit = read_string(payload, &mut offset)?;
        entries.push(DecodedTableMetadataEntry {
            id,
            x_scale,
            y_scale,
            value_scale,
            min_value_raw,
            max_value_raw,
            default_value_raw,
            key,
            x_label,
            y_label,
            z_label,
            unit,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
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

pub fn encode_output_test_directory_payload(entries: &[OutputTestDirectoryEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(entries.len() as u8);
    for entry in entries {
        out.push(entry.channel);
        out.push(output_group_code(entry.group).unwrap_or(0));
        out.extend_from_slice(&entry.default_pulse_ms.unwrap_or(u16::MAX).to_be_bytes());
        push_string(&mut out, entry.function);
        push_string(&mut out, entry.label);
    }
    out
}

pub fn decode_output_test_directory_payload(
    payload: &[u8],
) -> Result<Vec<DecodedOutputTestDirectoryEntry>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if payload.len() < offset + 4 {
            return Err(ProtocolError::MalformedPayload);
        }
        let channel = payload[offset];
        let group = output_group_key(payload[offset + 1])
            .ok_or(ProtocolError::MalformedPayload)?
            .to_string();
        let default_pulse_ms = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]);
        offset += 4;
        let function = read_string(payload, &mut offset)?;
        let label = read_string(payload, &mut offset)?;
        entries.push(DecodedOutputTestDirectoryEntry {
            channel,
            function,
            label,
            group,
            default_pulse_ms: (default_pulse_ms != u16::MAX).then_some(default_pulse_ms),
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
}

pub fn encode_sensor_raw_payload(adc: u16, voltage: f32) -> Vec<u8> {
    let mut out = Vec::with_capacity(6);
    out.extend_from_slice(&adc.to_be_bytes());
    out.extend_from_slice(&voltage.to_be_bytes());
    out
}

pub fn decode_sensor_raw_payload(payload: &[u8]) -> Result<DecodedSensorRaw, ProtocolError> {
    if payload.len() != 6 {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(DecodedSensorRaw {
        adc: u16::from_be_bytes([payload[0], payload[1]]),
        voltage: f32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]),
    })
}

pub fn encode_sensor_raw_directory_payload(entries: &[SensorRawDirectoryEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(entries.len() as u8);
    for entry in entries {
        out.push(entry.channel);
        out.extend_from_slice(&entry.expected_min_mv.to_be_bytes());
        out.extend_from_slice(&entry.expected_max_mv.to_be_bytes());
        out.extend_from_slice(&entry.fault_low_mv.to_be_bytes());
        out.extend_from_slice(&entry.fault_high_mv.to_be_bytes());
        push_string(&mut out, entry.key);
        push_string(&mut out, entry.label);
        push_string(&mut out, entry.unit);
        push_string(&mut out, entry.pin_id);
        push_string(&mut out, entry.pin_label);
    }
    out
}

pub fn decode_sensor_raw_directory_payload(
    payload: &[u8],
) -> Result<Vec<DecodedSensorRawDirectoryEntry>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if payload.len() < offset + 9 {
            return Err(ProtocolError::MalformedPayload);
        }
        let channel = payload[offset];
        let expected_min_mv = u16::from_be_bytes([payload[offset + 1], payload[offset + 2]]);
        let expected_max_mv = u16::from_be_bytes([payload[offset + 3], payload[offset + 4]]);
        let fault_low_mv = u16::from_be_bytes([payload[offset + 5], payload[offset + 6]]);
        let fault_high_mv = u16::from_be_bytes([payload[offset + 7], payload[offset + 8]]);
        offset += 9;
        let key = read_string(payload, &mut offset)?;
        let label = read_string(payload, &mut offset)?;
        let unit = read_string(payload, &mut offset)?;
        let pin_id = read_string(payload, &mut offset)?;
        let pin_label = read_string(payload, &mut offset)?;
        entries.push(DecodedSensorRawDirectoryEntry {
            channel,
            key,
            label,
            unit,
            pin_id,
            pin_label,
            expected_min_mv,
            expected_max_mv,
            fault_low_mv,
            fault_high_mv,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
}

pub fn encode_freeze_frames_payload(entries: &[FreezeFrame]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(entries.len() as u8);
    for entry in entries {
        push_string(&mut out, entry.code);
        push_string(&mut out, entry.label);
        push_string(&mut out, entry.reason);
        out.extend_from_slice(&entry.rev_counter.to_be_bytes());
        out.extend_from_slice(&entry.rpm.to_be_bytes());
        out.extend_from_slice(&entry.map_kpa_x10.to_be_bytes());
        out.extend_from_slice(&entry.tps_pct_x100.to_be_bytes());
        out.extend_from_slice(&entry.coolant_c_x10.to_be_bytes());
        out.extend_from_slice(&entry.lambda_x10000.to_be_bytes());
        out.extend_from_slice(&entry.vbatt_x100.to_be_bytes());
        out.push(entry.gear);
    }
    out
}

pub fn decode_freeze_frames_payload(
    payload: &[u8],
) -> Result<Vec<DecodedFreezeFrame>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let code = read_string(payload, &mut offset)?;
        let label = read_string(payload, &mut offset)?;
        let reason = read_string(payload, &mut offset)?;
        if payload.len() < offset + 17 {
            return Err(ProtocolError::MalformedPayload);
        }
        let rev_counter = u32::from_be_bytes([
            payload[offset],
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
        ]);
        let rpm = u16::from_be_bytes([payload[offset + 4], payload[offset + 5]]);
        let map_kpa = f64::from(u16::from_be_bytes([
            payload[offset + 6],
            payload[offset + 7],
        ])) / 10.0;
        let tps_pct = f64::from(u16::from_be_bytes([
            payload[offset + 8],
            payload[offset + 9],
        ])) / 100.0;
        let coolant_c = f64::from(i16::from_be_bytes([
            payload[offset + 10],
            payload[offset + 11],
        ])) / 10.0;
        let lambda = f64::from(u16::from_be_bytes([
            payload[offset + 12],
            payload[offset + 13],
        ])) / 10000.0;
        let vbatt = f64::from(u16::from_be_bytes([
            payload[offset + 14],
            payload[offset + 15],
        ])) / 100.0;
        let gear = payload[offset + 16];
        offset += 17;

        entries.push(DecodedFreezeFrame {
            code,
            label,
            reason,
            rev_counter,
            rpm,
            map_kpa,
            tps_pct,
            coolant_c,
            lambda,
            vbatt,
            gear,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
}

pub fn encode_trigger_decoder_directory_payload(entries: &[TriggerDecoderPreset]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(entries.len() as u8);
    for entry in entries {
        out.push(entry.requires_secondary as u8);
        out.push(entry.supports_sequential as u8);
        out.extend_from_slice(&entry.expected_engine_cycle_deg.to_be_bytes());
        push_string(&mut out, entry.key);
        push_string(&mut out, entry.label);
        push_string(&mut out, entry.family);
        push_string(&mut out, entry.decoder);
        push_string(&mut out, entry.pattern_kind);
        push_string(&mut out, entry.primary_input_label);
        push_string(&mut out, entry.secondary_input_label.unwrap_or(""));
        push_string(&mut out, entry.primary_sensor_kind);
        push_string(&mut out, entry.secondary_sensor_kind.unwrap_or(""));
        push_string(&mut out, entry.edge_policy);
        push_string(&mut out, entry.sync_strategy);
        push_string(&mut out, entry.primary_pattern_hint);
        push_string(&mut out, entry.secondary_pattern_hint.unwrap_or(""));
        push_string(&mut out, entry.reference_description);
    }
    out
}

pub fn decode_trigger_decoder_directory_payload(
    payload: &[u8],
) -> Result<Vec<DecodedTriggerDecoderPreset>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if payload.len() < offset + 4 {
            return Err(ProtocolError::MalformedPayload);
        }
        let requires_secondary = payload[offset] != 0;
        let supports_sequential = payload[offset + 1] != 0;
        let expected_engine_cycle_deg =
            u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]);
        offset += 4;
        let key = read_string(payload, &mut offset)?;
        let label = read_string(payload, &mut offset)?;
        let family = read_string(payload, &mut offset)?;
        let decoder = read_string(payload, &mut offset)?;
        let pattern_kind = read_string(payload, &mut offset)?;
        let primary_input_label = read_string(payload, &mut offset)?;
        let secondary_input_label = read_string(payload, &mut offset)?;
        let primary_sensor_kind = read_string(payload, &mut offset)?;
        let secondary_sensor_kind = read_string(payload, &mut offset)?;
        let edge_policy = read_string(payload, &mut offset)?;
        let sync_strategy = read_string(payload, &mut offset)?;
        let primary_pattern_hint = read_string(payload, &mut offset)?;
        let secondary_pattern_hint = read_string(payload, &mut offset)?;
        let reference_description = read_string(payload, &mut offset)?;
        entries.push(DecodedTriggerDecoderPreset {
            key,
            label,
            family,
            decoder,
            pattern_kind,
            primary_input_label,
            secondary_input_label: (!secondary_input_label.is_empty())
                .then_some(secondary_input_label),
            primary_sensor_kind,
            secondary_sensor_kind: (!secondary_sensor_kind.is_empty())
                .then_some(secondary_sensor_kind),
            edge_policy,
            sync_strategy,
            primary_pattern_hint,
            secondary_pattern_hint: (!secondary_pattern_hint.is_empty())
                .then_some(secondary_pattern_hint),
            reference_description,
            expected_engine_cycle_deg,
            requires_secondary,
            supports_sequential,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
}

pub fn encode_trigger_capture_payload(entry: &TriggerCapture) -> Vec<u8> {
    let mut out = Vec::new();
    push_string(&mut out, entry.preset_key);
    push_string(&mut out, entry.preset_label);
    push_string(&mut out, entry.sync_state);
    out.extend_from_slice(&entry.trigger_rpm.to_be_bytes());
    out.extend_from_slice(&entry.sync_loss_counter.to_be_bytes());
    out.extend_from_slice(&entry.synced_cycles.to_be_bytes());
    out.extend_from_slice(&entry.engine_cycle_deg.to_be_bytes());
    out.extend_from_slice(&entry.capture_window_us.to_be_bytes());
    out.extend_from_slice(&entry.sample_period_us.to_be_bytes());
    push_string(&mut out, entry.primary_label);
    push_string(&mut out, entry.secondary_label.unwrap_or(""));
    out.extend_from_slice(&entry.tooth_count.to_be_bytes());
    out.push(entry.sync_gap_tooth_count);
    out.extend_from_slice(&entry.primary_edge_count.to_be_bytes());
    out.extend_from_slice(&entry.secondary_edge_count.to_be_bytes());
    out.extend_from_slice(&(entry.primary_samples.len() as u16).to_be_bytes());
    out.extend_from_slice(&entry.primary_samples);
    out.extend_from_slice(&entry.secondary_samples);
    out
}

pub fn encode_trigger_tooth_log_payload(entry: &TriggerToothLog) -> Vec<u8> {
    let mut out = Vec::new();
    push_string(&mut out, entry.preset_key);
    push_string(&mut out, entry.preset_label);
    push_string(&mut out, entry.sync_state);
    out.extend_from_slice(&entry.trigger_rpm.to_be_bytes());
    out.extend_from_slice(&entry.engine_cycle_deg.to_be_bytes());
    push_string(&mut out, entry.primary_label);
    push_string(&mut out, entry.secondary_label.unwrap_or(""));
    out.extend_from_slice(&entry.reference_event_index.to_be_bytes());
    out.extend_from_slice(&entry.dominant_gap_ratio.to_be_bytes());
    out.extend_from_slice(&(entry.tooth_intervals_us.len() as u16).to_be_bytes());
    for interval in &entry.tooth_intervals_us {
        out.extend_from_slice(&interval.to_be_bytes());
    }
    out.extend_from_slice(&(entry.secondary_event_indexes.len() as u8).to_be_bytes());
    for index in &entry.secondary_event_indexes {
        out.extend_from_slice(&index.to_be_bytes());
    }
    out
}

pub fn decode_trigger_capture_payload(
    payload: &[u8],
) -> Result<DecodedTriggerCapture, ProtocolError> {
    let mut offset = 0usize;
    let preset_key = read_string(payload, &mut offset)?;
    let preset_label = read_string(payload, &mut offset)?;
    let sync_state = read_string(payload, &mut offset)?;
    if payload.len() < offset + 19 {
        return Err(ProtocolError::MalformedPayload);
    }
    let trigger_rpm = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let sync_loss_counter = u32::from_be_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
    ]);
    offset += 4;
    let synced_cycles = u32::from_be_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
    ]);
    offset += 4;
    let engine_cycle_deg = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let capture_window_us = u32::from_be_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
    ]);
    offset += 4;
    let sample_period_us = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let primary_label = read_string(payload, &mut offset)?;
    let secondary_label = read_string(payload, &mut offset)?;
    if payload.len() < offset + 7 {
        return Err(ProtocolError::MalformedPayload);
    }
    let tooth_count = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let sync_gap_tooth_count = payload[offset];
    offset += 1;
    let primary_edge_count = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let secondary_edge_count = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let sample_count = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
    offset += 2;
    if payload.len() < offset + sample_count * 2 {
        return Err(ProtocolError::MalformedPayload);
    }
    let primary_samples = payload[offset..offset + sample_count].to_vec();
    offset += sample_count;
    let secondary_samples = payload[offset..offset + sample_count].to_vec();
    offset += sample_count;

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(DecodedTriggerCapture {
        preset_key,
        preset_label,
        sync_state,
        trigger_rpm,
        sync_loss_counter,
        synced_cycles,
        engine_cycle_deg,
        capture_window_us,
        sample_period_us,
        primary_label,
        secondary_label: (!secondary_label.is_empty()).then_some(secondary_label),
        tooth_count,
        sync_gap_tooth_count,
        primary_edge_count,
        secondary_edge_count,
        primary_samples,
        secondary_samples,
    })
}

pub fn decode_trigger_tooth_log_payload(
    payload: &[u8],
) -> Result<DecodedTriggerToothLog, ProtocolError> {
    let mut offset = 0usize;
    let preset_key = read_string(payload, &mut offset)?;
    let preset_label = read_string(payload, &mut offset)?;
    let sync_state = read_string(payload, &mut offset)?;
    if payload.len() < offset + 10 {
        return Err(ProtocolError::MalformedPayload);
    }
    let trigger_rpm = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let engine_cycle_deg = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let primary_label = read_string(payload, &mut offset)?;
    let secondary_label = read_string(payload, &mut offset)?;
    if payload.len() < offset + 8 {
        return Err(ProtocolError::MalformedPayload);
    }
    let reference_event_index = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    offset += 2;
    let dominant_gap_ratio = f32::from_be_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
    ]);
    offset += 4;
    let tooth_count = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
    offset += 2;
    if payload.len() < offset + (tooth_count * 4) + 1 {
        return Err(ProtocolError::MalformedPayload);
    }
    let mut tooth_intervals_us = Vec::with_capacity(tooth_count);
    for _ in 0..tooth_count {
        tooth_intervals_us.push(u32::from_be_bytes([
            payload[offset],
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
        ]));
        offset += 4;
    }
    let secondary_count = payload[offset] as usize;
    offset += 1;
    if payload.len() < offset + (secondary_count * 2) {
        return Err(ProtocolError::MalformedPayload);
    }
    let mut secondary_event_indexes = Vec::with_capacity(secondary_count);
    for _ in 0..secondary_count {
        secondary_event_indexes.push(u16::from_be_bytes([payload[offset], payload[offset + 1]]));
        offset += 2;
    }
    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(DecodedTriggerToothLog {
        preset_key,
        preset_label,
        sync_state,
        trigger_rpm,
        engine_cycle_deg,
        primary_label,
        secondary_label: (!secondary_label.is_empty()).then_some(secondary_label),
        reference_event_index,
        dominant_gap_ratio,
        tooth_intervals_us,
        secondary_event_indexes,
    })
}

pub fn encode_can_template_directory_payload(entries: &[CanTemplateDirectoryEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(entries.len() as u8);
    for entry in entries {
        out.push(entry.id);
        out.extend_from_slice(&entry.can_id.to_be_bytes());
        out.push(entry.extended_id as u8);
        out.push(entry.dlc);
        push_string(&mut out, entry.key);
        push_string(&mut out, entry.label);
        push_string(&mut out, entry.direction);
        push_string(&mut out, entry.category);
    }
    out
}

pub fn decode_can_template_directory_payload(
    payload: &[u8],
) -> Result<Vec<DecodedCanTemplateDirectoryEntry>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };
    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);

    for _ in 0..count {
        if payload.len() < offset + 7 {
            return Err(ProtocolError::MalformedPayload);
        }
        let id = payload[offset];
        let can_id = u32::from_be_bytes([
            payload[offset + 1],
            payload[offset + 2],
            payload[offset + 3],
            payload[offset + 4],
        ]);
        let extended_id = payload[offset + 5] != 0;
        let dlc = payload[offset + 6];
        offset += 7;
        let key = read_string(payload, &mut offset)?;
        let label = read_string(payload, &mut offset)?;
        let direction = read_string(payload, &mut offset)?;
        let category = read_string(payload, &mut offset)?;
        entries.push(DecodedCanTemplateDirectoryEntry {
            id,
            key,
            label,
            direction,
            category,
            can_id,
            extended_id,
            dlc,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
}

pub fn encode_can_signal_directory_payload(entries: &[CanSignalDirectoryEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(entries.len() as u8);
    for entry in entries {
        out.push(entry.id);
        out.push(entry.template_id);
        out.push(entry.start_bit);
        out.push(entry.bit_length);
        let mut flags = 0u8;
        if entry.signed {
            flags |= 0x01;
        }
        if entry.little_endian {
            flags |= 0x02;
        }
        out.push(flags);
        out.extend_from_slice(&entry.scale.to_be_bytes());
        out.extend_from_slice(&entry.offset.to_be_bytes());
        out.extend_from_slice(&entry.min.to_be_bytes());
        out.extend_from_slice(&entry.max.to_be_bytes());
        push_string(&mut out, entry.key);
        push_string(&mut out, entry.label);
        push_string(&mut out, entry.maps_to);
        push_string(&mut out, entry.unit);
    }
    out
}

pub fn decode_can_signal_directory_payload(
    payload: &[u8],
) -> Result<Vec<DecodedCanSignalDirectoryEntry>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };
    let mut offset = 1usize;
    let mut entries = Vec::with_capacity(count as usize);

    for _ in 0..count {
        if payload.len() < offset + 21 {
            return Err(ProtocolError::MalformedPayload);
        }
        let id = payload[offset];
        let template_id = payload[offset + 1];
        let start_bit = payload[offset + 2];
        let bit_length = payload[offset + 3];
        let flags = payload[offset + 4];
        let scale = f32::from_be_bytes([
            payload[offset + 5],
            payload[offset + 6],
            payload[offset + 7],
            payload[offset + 8],
        ]);
        let offset_value = f32::from_be_bytes([
            payload[offset + 9],
            payload[offset + 10],
            payload[offset + 11],
            payload[offset + 12],
        ]);
        let min = f32::from_be_bytes([
            payload[offset + 13],
            payload[offset + 14],
            payload[offset + 15],
            payload[offset + 16],
        ]);
        let max = f32::from_be_bytes([
            payload[offset + 17],
            payload[offset + 18],
            payload[offset + 19],
            payload[offset + 20],
        ]);
        offset += 21;
        let key = read_string(payload, &mut offset)?;
        let label = read_string(payload, &mut offset)?;
        let maps_to = read_string(payload, &mut offset)?;
        let unit = read_string(payload, &mut offset)?;
        entries.push(DecodedCanSignalDirectoryEntry {
            id,
            template_id,
            key,
            label,
            maps_to,
            start_bit,
            bit_length,
            signed: (flags & 0x01) != 0,
            little_endian: (flags & 0x02) != 0,
            scale,
            offset: offset_value,
            min,
            max,
            unit,
        });
    }

    if offset != payload.len() {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(entries)
}

pub fn encode_page_statuses_payload(statuses: &[ConfigPageStatus]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + statuses.len() * 15);
    out.push(statuses.len() as u8);
    for status in statuses {
        out.push(status.page_id);
        out.extend_from_slice(&status.ram_crc.to_be_bytes());
        out.extend_from_slice(&status.flash_crc.to_be_bytes());
        out.push(status.needs_burn as u8);
        out.extend_from_slice(&status.flash_generation.to_be_bytes());
        out.push(status.flash_valid as u8);
    }
    out
}

pub fn decode_page_statuses_payload(
    payload: &[u8],
) -> Result<Vec<DecodedPageStatus>, ProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(ProtocolError::MalformedPayload);
    };

    let legacy_len = 1 + count as usize * 10;
    let extended_len = 1 + count as usize * 15;
    let extended = if payload.len() == extended_len {
        true
    } else if payload.len() == legacy_len {
        false
    } else {
        return Err(ProtocolError::MalformedPayload);
    };

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
        let (flash_generation, flash_valid, step) = if extended {
            (
                u32::from_be_bytes([
                    payload[offset + 10],
                    payload[offset + 11],
                    payload[offset + 12],
                    payload[offset + 13],
                ]),
                payload[offset + 14] != 0,
                15,
            )
        } else {
            (0, true, 10)
        };
        statuses.push(DecodedPageStatus {
            page_id,
            ram_crc,
            flash_crc,
            needs_burn,
            flash_generation,
            flash_valid,
        });
        offset += step;
    }

    Ok(statuses)
}

pub fn encode_firmware_update_status_payload(status: &FirmwareUpdateStatusPayload) -> Vec<u8> {
    let mut out = Vec::with_capacity(19);
    out.push(status.state);
    out.push(status.active_bank);
    out.push(status.last_good_bank);
    out.push(status.pending_bank.unwrap_or(u8::MAX));
    out.push(status.boot_attempts);
    out.extend_from_slice(&status.rollback_counter.to_be_bytes());
    out.extend_from_slice(&status.candidate_size.to_be_bytes());
    out.extend_from_slice(&status.candidate_crc.to_be_bytes());
    out.extend_from_slice(&status.health_window_remaining_ms.to_be_bytes());
    out
}

pub fn decode_firmware_update_status_payload(
    payload: &[u8],
) -> Result<FirmwareUpdateStatusPayload, ProtocolError> {
    if payload.len() != 19 {
        return Err(ProtocolError::MalformedPayload);
    }

    Ok(FirmwareUpdateStatusPayload {
        state: payload[0],
        active_bank: payload[1],
        last_good_bank: payload[2],
        pending_bank: (payload[3] != u8::MAX).then_some(payload[3]),
        boot_attempts: payload[4],
        rollback_counter: u16::from_be_bytes([payload[5], payload[6]]),
        candidate_size: u32::from_be_bytes([payload[7], payload[8], payload[9], payload[10]]),
        candidate_crc: u32::from_be_bytes([payload[11], payload[12], payload[13], payload[14]]),
        health_window_remaining_ms: u32::from_be_bytes([
            payload[15],
            payload[16],
            payload[17],
            payload[18],
        ]),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlashResumeRequestPayload {
    pub session_id: Option<u32>,
}

pub fn encode_flash_resume_payload(session_id: u32, next_block: u32, rolling_crc: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    out.extend_from_slice(&session_id.to_be_bytes());
    out.extend_from_slice(&next_block.to_be_bytes());
    out.extend_from_slice(&rolling_crc.to_be_bytes());
    out
}

pub fn decode_flash_resume_payload(payload: &[u8]) -> Result<(u32, u32, u32), ProtocolError> {
    if payload.len() != 12 {
        return Err(ProtocolError::MalformedPayload);
    }
    let session_id = u32::from_be_bytes(payload[0..4].try_into().unwrap());
    let next_block = u32::from_be_bytes(payload[4..8].try_into().unwrap());
    let rolling_crc = u32::from_be_bytes(payload[8..12].try_into().unwrap());
    Ok((session_id, next_block, rolling_crc))
}

pub fn decode_flash_resume_request_payload(
    payload: &[u8],
) -> Result<FlashResumeRequestPayload, ProtocolError> {
    match payload.len() {
        0 => Ok(FlashResumeRequestPayload { session_id: None }),
        4 => Ok(FlashResumeRequestPayload {
            session_id: Some(u32::from_be_bytes(payload.try_into().unwrap())),
        }),
        _ => Err(ProtocolError::MalformedPayload),
    }
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

pub fn encode_log_status_payload(status: &LogStatusPayload) -> Vec<u8> {
    let mut out = Vec::with_capacity(20);
    out.push(status.active as u8);
    out.push(status.storage_present as u8);
    out.push(status.rtc_synced as u8);
    out.push(status.logbook_entries);
    out.extend_from_slice(&status.session_id.to_be_bytes());
    out.extend_from_slice(&status.elapsed_ms.to_be_bytes());
    out.extend_from_slice(&status.bytes_written.to_be_bytes());
    out.extend_from_slice(&status.block_count.to_be_bytes());
    out.extend_from_slice(&status.block_size.to_be_bytes());
    out
}

pub fn decode_log_status_payload(payload: &[u8]) -> Result<LogStatusPayload, ProtocolError> {
    if payload.len() != 20 {
        return Err(ProtocolError::MalformedPayload);
    }
    Ok(LogStatusPayload {
        active: payload[0] != 0,
        storage_present: payload[1] != 0,
        rtc_synced: payload[2] != 0,
        logbook_entries: payload[3],
        session_id: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
        elapsed_ms: u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]),
        bytes_written: u32::from_be_bytes([payload[12], payload[13], payload[14], payload[15]]),
        block_count: u16::from_be_bytes([payload[16], payload[17]]),
        block_size: u16::from_be_bytes([payload[18], payload[19]]),
    })
}

pub fn encode_logbook_summary_payload(summary: &LogbookSummaryPayload) -> Vec<u8> {
    let mut out = Vec::with_capacity(33);
    out.extend_from_slice(&summary.sessions.to_be_bytes());
    out.extend_from_slice(&summary.entries.to_be_bytes());
    out.extend_from_slice(&summary.total_elapsed_ms.to_be_bytes());
    out.extend_from_slice(&summary.total_bytes_written.to_be_bytes());
    out.extend_from_slice(&summary.last_session_id.to_be_bytes());
    out.extend_from_slice(&summary.last_elapsed_ms.to_be_bytes());
    out.extend_from_slice(&summary.last_bytes_written.to_be_bytes());
    out.extend_from_slice(&summary.last_block_count.to_be_bytes());
    out.push(summary.rtc_synced as u8);
    out.extend_from_slice(&summary.last_rtc_sync_ms.to_be_bytes());
    out
}

pub fn decode_logbook_summary_payload(
    payload: &[u8],
) -> Result<LogbookSummaryPayload, ProtocolError> {
    if payload.len() != 33 {
        return Err(ProtocolError::MalformedPayload);
    }
    Ok(LogbookSummaryPayload {
        sessions: u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]),
        entries: u16::from_be_bytes([payload[4], payload[5]]),
        total_elapsed_ms: u32::from_be_bytes([payload[6], payload[7], payload[8], payload[9]]),
        total_bytes_written: u32::from_be_bytes([
            payload[10],
            payload[11],
            payload[12],
            payload[13],
        ]),
        last_session_id: u32::from_be_bytes([payload[14], payload[15], payload[16], payload[17]]),
        last_elapsed_ms: u32::from_be_bytes([payload[18], payload[19], payload[20], payload[21]]),
        last_bytes_written: u32::from_be_bytes([
            payload[22],
            payload[23],
            payload[24],
            payload[25],
        ]),
        last_block_count: u16::from_be_bytes([payload[26], payload[27]]),
        rtc_synced: payload[28] != 0,
        last_rtc_sync_ms: u32::from_be_bytes([payload[29], payload[30], payload[31], payload[32]]),
    })
}

pub fn encode_log_block_payload(block_index: u16, total_blocks: u16, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(10 + data.len());
    out.extend_from_slice(&block_index.to_be_bytes());
    out.extend_from_slice(&total_blocks.to_be_bytes());
    out.extend_from_slice(&(data.len() as u16).to_be_bytes());
    out.extend_from_slice(&CRC32.checksum(data).to_be_bytes());
    out.extend_from_slice(data);
    out
}

pub fn decode_log_block_payload(payload: &[u8]) -> Result<DecodedLogBlockPayload, ProtocolError> {
    if payload.len() < 10 {
        return Err(ProtocolError::MalformedPayload);
    }
    let block_index = u16::from_be_bytes([payload[0], payload[1]]);
    let total_blocks = u16::from_be_bytes([payload[2], payload[3]]);
    let data_len = u16::from_be_bytes([payload[4], payload[5]]) as usize;
    if payload.len() != 10 + data_len {
        return Err(ProtocolError::MalformedPayload);
    }
    let payload_crc = u32::from_be_bytes([payload[6], payload[7], payload[8], payload[9]]);
    let data = payload[10..].to_vec();
    let actual_crc = CRC32.checksum(&data);
    if payload_crc != actual_crc {
        return Err(ProtocolError::MalformedPayload);
    }
    Ok(DecodedLogBlockPayload {
        block_index,
        total_blocks,
        payload_crc,
        payload: data,
    })
}

pub fn encode_sync_rtc_payload(epoch_ms: u64) -> Vec<u8> {
    epoch_ms.to_be_bytes().to_vec()
}

pub fn decode_sync_rtc_payload(payload: &[u8]) -> Result<u64, ProtocolError> {
    if payload.len() != 8 {
        return Err(ProtocolError::MalformedPayload);
    }
    Ok(u64::from_be_bytes([
        payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6],
        payload[7],
    ]))
}

pub fn decode_raw_table_payload(payload: &[u8]) -> Result<RawTablePayload, ProtocolError> {
    if payload.len() < 3 {
        return Err(ProtocolError::MalformedPayload);
    }

    let table_id = payload[0];
    let x_count = payload[1] as usize;
    let y_count = payload[2] as usize;
    let expected_words = x_count + y_count + (x_count * y_count);
    let expected_len = 3 + expected_words * 2;
    if payload.len() != expected_len {
        return Err(ProtocolError::MalformedPayload);
    }

    let mut offset = 3usize;
    let mut x_bins = Vec::with_capacity(x_count);
    for _ in 0..x_count {
        x_bins.push(u16::from_be_bytes([payload[offset], payload[offset + 1]]));
        offset += 2;
    }

    let mut y_bins = Vec::with_capacity(y_count);
    for _ in 0..y_count {
        y_bins.push(u16::from_be_bytes([payload[offset], payload[offset + 1]]));
        offset += 2;
    }

    let mut data = Vec::with_capacity(x_count * y_count);
    for _ in 0..(x_count * y_count) {
        data.push(u16::from_be_bytes([payload[offset], payload[offset + 1]]));
        offset += 2;
    }

    Ok(RawTablePayload {
        table_id,
        x_count: x_count as u8,
        y_count: y_count as u8,
        x_bins,
        y_bins,
        data,
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

fn output_group_key(code: u8) -> Option<&'static str> {
    match code {
        0x01 => Some("injectors"),
        0x02 => Some("coils"),
        0x03 => Some("aux"),
        0x04 => Some("valves"),
        _ => None,
    }
}

fn output_group_code(key: &str) -> Option<u8> {
    match key {
        "injectors" => Some(0x01),
        "coils" => Some(0x02),
        "aux" => Some(0x03),
        "valves" => Some(0x04),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::config::ConfigPageStatus;
    use crate::contract::{base_capabilities, Capability, FirmwareIdentity};
    use crate::diagnostics::FreezeFrame;
    use crate::io::{EcuFunction, ResolvedPinAssignment};
    use crate::network::{display_network_profile, MessageClass, ProductTrack, TransportLinkKind};
    use crate::pinmux::PinFunctionClass;
    use crate::trigger::{sample_trigger_capture, sample_trigger_tooth_log, TriggerDecoderPreset};
    use std::collections::BTreeMap;

    use super::{
        decode_ack_payload, decode_can_signal_directory_payload,
        decode_can_template_directory_payload, decode_capabilities_payload,
        decode_firmware_update_status_payload, decode_flash_resume_payload,
        decode_flash_resume_request_payload, decode_freeze_frames_payload, decode_identity_payload,
        decode_log_block_payload, decode_log_status_payload, decode_logbook_summary_payload,
        decode_nack_payload, decode_network_profile_payload, decode_output_test_directory_payload,
        decode_page_payload, decode_page_request, decode_page_statuses_payload,
        decode_pin_assignments_payload, decode_pin_directory_payload, decode_raw_table_payload,
        decode_sensor_raw_directory_payload, decode_sensor_raw_payload, decode_sync_rtc_payload,
        decode_table_metadata_payload, decode_trigger_capture_payload,
        decode_trigger_decoder_directory_payload, decode_trigger_tooth_log_payload,
        encode_ack_payload, encode_can_signal_directory_payload,
        encode_can_template_directory_payload, encode_capabilities_payload,
        encode_firmware_update_status_payload, encode_flash_resume_payload,
        encode_freeze_frames_payload, encode_identity_payload, encode_log_block_payload,
        encode_log_status_payload, encode_logbook_summary_payload, encode_nack_payload,
        encode_network_profile_payload, encode_output_test_directory_payload,
        encode_page_directory_payload, encode_page_payload, encode_page_request,
        encode_page_statuses_payload, encode_pin_assignments_payload, encode_pin_directory_payload,
        encode_sensor_raw_directory_payload, encode_sensor_raw_payload, encode_sync_rtc_payload,
        encode_table_directory_payload, encode_table_metadata_payload,
        encode_trigger_capture_payload, encode_trigger_decoder_directory_payload,
        encode_trigger_tooth_log_payload, CanSignalDirectoryEntry, CanTemplateDirectoryEntry, Cmd,
        DecodedPinAssignment, FirmwareUpdateStatusPayload, LogStatusPayload, LogbookSummaryPayload,
        OutputTestDirectoryEntry, Packet, ProtocolError, RawTablePayload, SensorRawDirectoryEntry,
    };
    use proptest::prelude::*;

    #[test]
    fn packet_roundtrip() {
        let packet = Packet::new(Cmd::Ping, vec![1, 2, 3, 4]);
        let bytes = packet.to_bytes();
        let parsed = Packet::from_bytes(&bytes).unwrap().unwrap();
        assert_eq!(parsed.0, packet);
    }

    proptest! {
        #[test]
        fn packet_roundtrip_property(
            payload in prop::collection::vec(any::<u8>(), 0..512),
            cmd in prop_oneof![
                Just(Cmd::Ping),
                Just(Cmd::GetLiveData),
                Just(Cmd::ReadPage),
                Just(Cmd::WritePage),
                Just(Cmd::ReadTable),
                Just(Cmd::WriteTable),
                Just(Cmd::GetTriggerCapture),
            ],
        ) {
            let packet = Packet::new(cmd, payload.clone());
            let bytes = packet.to_bytes();
            let parsed = Packet::from_bytes(&bytes)
                .expect("parse result")
                .expect("complete packet");
            prop_assert_eq!(parsed.0.cmd, cmd);
            prop_assert_eq!(parsed.0.payload, payload);
            prop_assert_eq!(parsed.1, bytes.len());
        }

        #[test]
        fn page_payload_decode_rejects_crc_corruption(
            page_id in any::<u8>(),
            payload_bytes in prop::collection::vec(any::<u8>(), 0..384),
        ) {
            let mut payload = encode_page_payload(page_id, &payload_bytes);
            prop_assume!(!payload.is_empty());
            let last = payload.len() - 1;
            payload[last] ^= 0xA5;
            let result = decode_page_payload(&payload);
            prop_assert!(matches!(result, Err(ProtocolError::MalformedPayload)));
        }
    }

    #[test]
    fn identity_payload_roundtrip() {
        let payload =
            encode_identity_payload(&FirmwareIdentity::ecu_v1(), &base_capabilities(false));
        let decoded = decode_identity_payload(&payload).unwrap();
        assert_eq!(decoded.protocol_version, 1);
        assert_eq!(decoded.schema_version, 1);
        assert_eq!(decoded.board_id, "st-ecu-v1");
        assert_eq!(decoded.reset_reason.as_deref(), Some("power_on"));
        assert!(!decoded.capabilities.is_empty());
    }

    #[test]
    fn identity_payload_decode_supports_legacy_shape_without_reset_reason() {
        let identity = FirmwareIdentity::ecu_v1();
        let capabilities = base_capabilities(false);
        let mut payload = Vec::new();
        payload.push(identity.protocol_version);
        payload.extend_from_slice(&identity.schema_version.to_be_bytes());
        payload.push(capabilities.len() as u8);
        super::push_string(&mut payload, identity.firmware_id);
        super::push_string(&mut payload, identity.firmware_semver);
        super::push_string(&mut payload, identity.board_id);
        super::push_string(&mut payload, identity.serial);
        super::push_string(&mut payload, identity.signature);
        for capability in capabilities {
            payload.push(capability.code());
        }

        let decoded = decode_identity_payload(&payload).unwrap();
        assert_eq!(decoded.board_id, "st-ecu-v1");
        assert_eq!(decoded.reset_reason, None);
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
        let metadata = encode_table_metadata_payload();
        assert!(pages.len() > 4);
        assert!(tables.len() > 4);
        assert!(metadata.len() > tables.len());
    }

    #[test]
    fn table_metadata_roundtrip() {
        let payload = encode_table_metadata_payload();
        let entries = decode_table_metadata_payload(&payload).unwrap();
        let ignition = entries.iter().find(|entry| entry.id == 0x01).unwrap();
        let ve = entries.iter().find(|entry| entry.id == 0x00).unwrap();

        assert_eq!(ignition.unit, "deg");
        assert_eq!(ignition.min_value_raw, -200);
        assert_eq!(ignition.value_scale, 10);
        assert_eq!(ve.unit, "%");
        assert_eq!(ve.min_value_raw, 0);
        assert_eq!(ve.y_label, "MAP");
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
    fn raw_table_payload_roundtrip() {
        let table = RawTablePayload {
            table_id: 0x03,
            x_count: 4,
            y_count: 2,
            x_bins: vec![500, 1000, 1500, 2000],
            y_bins: vec![200, 800],
            data: vec![10, 20, 30, 40, 50, 60, 70, 80],
        };
        let payload = table.to_payload();
        let decoded = decode_raw_table_payload(&payload).unwrap();
        assert_eq!(decoded, table);
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
    fn firmware_update_status_payload_roundtrip() {
        let payload = encode_firmware_update_status_payload(&FirmwareUpdateStatusPayload {
            state: 4,
            active_bank: 1,
            last_good_bank: 1,
            pending_bank: Some(2),
            boot_attempts: 1,
            rollback_counter: 2,
            candidate_size: 182_144,
            candidate_crc: 0x12AB34CD,
            health_window_remaining_ms: 28_500,
        });
        let decoded = decode_firmware_update_status_payload(&payload).unwrap();
        assert_eq!(decoded.state, 4);
        assert_eq!(decoded.active_bank, 1);
        assert_eq!(decoded.pending_bank, Some(2));
        assert_eq!(decoded.candidate_size, 182_144);
        assert_eq!(decoded.candidate_crc, 0x12AB34CD);
        assert_eq!(decoded.health_window_remaining_ms, 28_500);
    }

    #[test]
    fn flash_resume_payload_roundtrip() {
        let payload = encode_flash_resume_payload(0x0102_0304, 7, 0xAABB_CCDD);
        let decoded = decode_flash_resume_payload(&payload).unwrap();
        assert_eq!(decoded, (0x0102_0304, 7, 0xAABB_CCDD));
    }

    #[test]
    fn flash_resume_request_payload_accepts_empty_or_session_id() {
        let empty = decode_flash_resume_request_payload(&[]).unwrap();
        assert_eq!(empty.session_id, None);

        let session = decode_flash_resume_request_payload(&0x0BAD_F00Du32.to_be_bytes()).unwrap();
        assert_eq!(session.session_id, Some(0x0BAD_F00D));

        assert!(decode_flash_resume_request_payload(&[0xAA]).is_err());
    }

    #[test]
    fn log_status_payload_roundtrip() {
        let payload = encode_log_status_payload(&LogStatusPayload {
            active: true,
            storage_present: true,
            rtc_synced: false,
            logbook_entries: 4,
            session_id: 17,
            elapsed_ms: 2_450,
            bytes_written: 98_112,
            block_count: 12,
            block_size: 256,
        });
        let decoded = decode_log_status_payload(&payload).unwrap();
        assert!(decoded.active);
        assert_eq!(decoded.logbook_entries, 4);
        assert_eq!(decoded.session_id, 17);
        assert_eq!(decoded.bytes_written, 98_112);
        assert_eq!(decoded.block_count, 12);
        assert_eq!(decoded.block_size, 256);
    }

    #[test]
    fn log_block_payload_roundtrip() {
        let payload = encode_log_block_payload(3, 9, &[0xAA, 0x10, 0x99, 0x01]);
        let decoded = decode_log_block_payload(&payload).unwrap();
        assert_eq!(decoded.block_index, 3);
        assert_eq!(decoded.total_blocks, 9);
        assert_eq!(decoded.payload, vec![0xAA, 0x10, 0x99, 0x01]);
    }

    #[test]
    fn logbook_summary_payload_roundtrip() {
        let payload = encode_logbook_summary_payload(&LogbookSummaryPayload {
            sessions: 6,
            entries: 6,
            total_elapsed_ms: 120_450,
            total_bytes_written: 1_248_900,
            last_session_id: 17,
            last_elapsed_ms: 15_240,
            last_bytes_written: 186_420,
            last_block_count: 38,
            rtc_synced: true,
            last_rtc_sync_ms: 9_540_200,
        });
        let decoded = decode_logbook_summary_payload(&payload).unwrap();
        assert_eq!(decoded.sessions, 6);
        assert_eq!(decoded.entries, 6);
        assert_eq!(decoded.total_elapsed_ms, 120_450);
        assert_eq!(decoded.total_bytes_written, 1_248_900);
        assert_eq!(decoded.last_session_id, 17);
        assert_eq!(decoded.last_block_count, 38);
        assert!(decoded.rtc_synced);
    }

    #[test]
    fn sync_rtc_payload_roundtrip() {
        let payload = encode_sync_rtc_payload(1_710_000_123_456);
        let decoded = decode_sync_rtc_payload(&payload).unwrap();
        assert_eq!(decoded, 1_710_000_123_456);
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
    fn output_test_directory_payload_roundtrip() {
        let payload = encode_output_test_directory_payload(&[
            OutputTestDirectoryEntry {
                channel: 0,
                function: "injector_1",
                label: "Injector 1",
                group: "injectors",
                default_pulse_ms: Some(5),
            },
            OutputTestDirectoryEntry {
                channel: 24,
                function: "idle_control",
                label: "Idle Valve",
                group: "valves",
                default_pulse_ms: None,
            },
        ]);
        let decoded = decode_output_test_directory_payload(&payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].group, "injectors");
        assert_eq!(decoded[0].default_pulse_ms, Some(5));
        assert_eq!(decoded[1].channel, 24);
        assert_eq!(decoded[1].label, "Idle Valve");
    }

    #[test]
    fn sensor_raw_payload_roundtrip() {
        let payload = encode_sensor_raw_payload(32123, 2.412);
        let decoded = decode_sensor_raw_payload(&payload).unwrap();

        assert_eq!(decoded.adc, 32123);
        assert!((decoded.voltage - 2.412).abs() < 0.0001);
    }

    #[test]
    fn sensor_raw_directory_payload_roundtrip() {
        let payload = encode_sensor_raw_directory_payload(&[
            SensorRawDirectoryEntry {
                channel: 0,
                key: "clt",
                label: "Coolant Temp",
                unit: "degC",
                pin_id: "PC1",
                pin_label: "CLT",
                expected_min_mv: 400,
                expected_max_mv: 2800,
                fault_low_mv: 100,
                fault_high_mv: 3200,
            },
            SensorRawDirectoryEntry {
                channel: 4,
                key: "vbatt",
                label: "Battery Sense",
                unit: "V",
                pin_id: "PA4",
                pin_label: "VBATT",
                expected_min_mv: 1800,
                expected_max_mv: 2800,
                fault_low_mv: 500,
                fault_high_mv: 3200,
            },
        ]);

        let decoded = decode_sensor_raw_directory_payload(&payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].key, "clt");
        assert_eq!(decoded[1].pin_label, "VBATT");
        assert_eq!(decoded[1].fault_high_mv, 3200);
    }

    #[test]
    fn freeze_frames_payload_roundtrip() {
        let payload = encode_freeze_frames_payload(&[
            FreezeFrame {
                code: "P0118",
                label: "Coolant Temp Sensor High",
                reason: "sensor_plausibility",
                rev_counter: 48_231,
                rpm: 2_840,
                map_kpa_x10: 572,
                tps_pct_x100: 1460,
                coolant_c_x10: -318,
                lambda_x10000: 10_720,
                vbatt_x100: 1418,
                gear: 3,
            },
            FreezeFrame {
                code: "P0193",
                label: "Fuel Pressure Sensor High",
                reason: "pressure_range_high",
                rev_counter: 50_984,
                rpm: 4_125,
                map_kpa_x10: 1834,
                tps_pct_x100: 6230,
                coolant_c_x10: 862,
                lambda_x10000: 9180,
                vbatt_x100: 1394,
                gear: 4,
            },
        ]);

        let decoded = decode_freeze_frames_payload(&payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].code, "P0118");
        assert!((decoded[0].coolant_c + 31.8).abs() < 0.1);
        assert_eq!(decoded[1].gear, 4);
    }

    #[test]
    fn trigger_decoder_directory_payload_roundtrip() {
        let payload = encode_trigger_decoder_directory_payload(&[
            TriggerDecoderPreset {
                key: "generic_60_2",
                label: "Generic 60-2 + Home",
                family: "Universal Missing-Tooth",
                decoder: "missing_tooth_60_2",
                pattern_kind: "missing_tooth",
                primary_input_label: "Crank VR/Hall",
                secondary_input_label: Some("Cam Home"),
                primary_sensor_kind: "vr_or_hall",
                secondary_sensor_kind: Some("hall_or_optical"),
                edge_policy: "configurable",
                sync_strategy: "missing_tooth_plus_home",
                primary_pattern_hint: "60-2 crank wheel on the primary input",
                secondary_pattern_hint: Some("Single home or cam-sync event every 720 deg"),
                reference_description: "Locks on the missing-tooth gap and confirms engine phase from the home input.",
                expected_engine_cycle_deg: 720,
                requires_secondary: true,
                supports_sequential: true,
            },
            TriggerDecoderPreset {
                key: "honda_k20_12_1",
                label: "Honda K20 / K24",
                family: "Honda K-Series",
                decoder: "oem_honda_k_12_1",
                pattern_kind: "oem_pattern",
                primary_input_label: "Crank (CKP)",
                secondary_input_label: Some("Cam / TDC (CMP)"),
                primary_sensor_kind: "hall",
                secondary_sensor_kind: Some("hall"),
                edge_policy: "decoder_defined",
                sync_strategy: "ckp_plus_cmp_phase",
                primary_pattern_hint: "12 CKP windows on the crank pattern",
                secondary_pattern_hint: Some("Honda K cam and TDC phase windows"),
                reference_description: "Uses CKP window timing plus CMP phase windows to identify the full engine cycle.",
                expected_engine_cycle_deg: 720,
                requires_secondary: true,
                supports_sequential: true,
            },
        ]);

        let decoded = decode_trigger_decoder_directory_payload(&payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].pattern_kind, "missing_tooth");
        assert_eq!(decoded[1].key, "honda_k20_12_1");
        assert_eq!(decoded[0].primary_sensor_kind, "vr_or_hall");
        assert_eq!(decoded[0].sync_strategy, "missing_tooth_plus_home");
        assert_eq!(
            decoded[0].primary_pattern_hint,
            "60-2 crank wheel on the primary input"
        );
        assert_eq!(decoded[1].expected_engine_cycle_deg, 720);
        assert_eq!(
            decoded[1].secondary_input_label.as_deref(),
            Some("Cam / TDC (CMP)")
        );
        assert_eq!(
            decoded[1].reference_description,
            "Uses CKP window timing plus CMP phase windows to identify the full engine cycle."
        );
    }

    #[test]
    fn trigger_capture_payload_roundtrip() {
        let payload = encode_trigger_capture_payload(&sample_trigger_capture());
        let decoded = decode_trigger_capture_payload(&payload).unwrap();

        assert_eq!(decoded.preset_key, "honda_k20_12_1");
        assert_eq!(decoded.sync_state, "locked");
        assert_eq!(decoded.trigger_rpm, 862);
        assert_eq!(
            decoded.primary_samples.len(),
            decoded.secondary_samples.len()
        );
        assert!(decoded.primary_samples.iter().any(|sample| *sample == 1));
    }

    #[test]
    fn trigger_tooth_log_payload_roundtrip() {
        let payload = encode_trigger_tooth_log_payload(&sample_trigger_tooth_log());
        let decoded = decode_trigger_tooth_log_payload(&payload).unwrap();

        assert_eq!(decoded.preset_key, "honda_k20_12_1");
        assert_eq!(decoded.sync_state, "locked");
        assert_eq!(decoded.reference_event_index, 2);
        assert_eq!(decoded.tooth_intervals_us.len(), 12);
        assert_eq!(decoded.secondary_event_indexes, vec![2, 8]);
        assert!(decoded
            .tooth_intervals_us
            .iter()
            .all(|interval| *interval >= 697 && *interval <= 702));
    }

    #[test]
    fn can_template_directory_payload_roundtrip() {
        let payload = encode_can_template_directory_payload(&[
            CanTemplateDirectoryEntry {
                id: 1,
                key: "st_engine_stream_tx",
                label: "ST Engine Stream TX",
                direction: "tx",
                category: "stream",
                can_id: 0x7E0,
                extended_id: false,
                dlc: 8,
            },
            CanTemplateDirectoryEntry {
                id: 2,
                key: "st_tcu_req_rx",
                label: "ST TCU Request RX",
                direction: "rx",
                category: "integration",
                can_id: 0x620,
                extended_id: false,
                dlc: 8,
            },
        ]);

        let decoded = decode_can_template_directory_payload(&payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].id, 1);
        assert_eq!(decoded[0].direction, "tx");
        assert_eq!(decoded[1].key, "st_tcu_req_rx");
        assert_eq!(decoded[1].can_id, 0x620);
    }

    #[test]
    fn can_signal_directory_payload_roundtrip() {
        let payload = encode_can_signal_directory_payload(&[
            CanSignalDirectoryEntry {
                id: 11,
                template_id: 2,
                key: "tcu_torque_reduction_pct",
                label: "TCU Torque Reduction",
                maps_to: "torque_reduction_pct",
                start_bit: 0,
                bit_length: 8,
                signed: false,
                little_endian: true,
                scale: 1.0,
                offset: 0.0,
                min: 0.0,
                max: 100.0,
                unit: "%",
            },
            CanSignalDirectoryEntry {
                id: 12,
                template_id: 2,
                key: "tcu_target_gear",
                label: "TCU Target Gear",
                maps_to: "gear_target",
                start_bit: 8,
                bit_length: 8,
                signed: false,
                little_endian: true,
                scale: 1.0,
                offset: 0.0,
                min: 0.0,
                max: 10.0,
                unit: "gear",
            },
        ]);

        let decoded = decode_can_signal_directory_payload(&payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].template_id, 2);
        assert_eq!(decoded[0].maps_to, "torque_reduction_pct");
        assert_eq!(decoded[0].unit, "%");
        assert_eq!(decoded[1].start_bit, 8);
        assert_eq!(decoded[1].bit_length, 8);
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

    #[test]
    fn cmd_values_are_stable_for_shared_contract() {
        assert_eq!(Cmd::Ping as u8, 0x01);
        assert_eq!(Cmd::Pong as u8, 0x02);
        assert_eq!(Cmd::GetVersion as u8, 0x03);
        assert_eq!(Cmd::VersionResponse as u8, 0x04);
        assert_eq!(Cmd::GetCapabilities as u8, 0x05);
        assert_eq!(Cmd::Capabilities as u8, 0x06);
        assert_eq!(Cmd::GetLiveData as u8, 0x07);
        assert_eq!(Cmd::LiveData as u8, 0x08);

        assert_eq!(Cmd::ReadPage as u8, 0x20);
        assert_eq!(Cmd::PageData as u8, 0x21);
        assert_eq!(Cmd::WritePage as u8, 0x22);
        assert_eq!(Cmd::BurnPage as u8, 0x23);
        assert_eq!(Cmd::GetPageDirectory as u8, 0x24);
        assert_eq!(Cmd::PageDirectory as u8, 0x25);
        assert_eq!(Cmd::GetTableDirectory as u8, 0x26);
        assert_eq!(Cmd::TableDirectory as u8, 0x27);
        assert_eq!(Cmd::GetPinDirectory as u8, 0x28);
        assert_eq!(Cmd::PinDirectory as u8, 0x29);
        assert_eq!(Cmd::GetTableMetadata as u8, 0x2A);
        assert_eq!(Cmd::TableMetadata as u8, 0x2B);
        assert_eq!(Cmd::GetPageStatuses as u8, 0x2C);
        assert_eq!(Cmd::PageStatuses as u8, 0x2D);
        assert_eq!(Cmd::GetNetworkProfile as u8, 0x2E);
        assert_eq!(Cmd::NetworkProfile as u8, 0x2F);

        assert_eq!(Cmd::ReadTable as u8, 0x30);
        assert_eq!(Cmd::TableData as u8, 0x31);
        assert_eq!(Cmd::WriteTable as u8, 0x32);
        assert_eq!(Cmd::WriteCell as u8, 0x33);
        assert_eq!(Cmd::ReadCurve as u8, 0x34);
        assert_eq!(Cmd::CurveData as u8, 0x35);
        assert_eq!(Cmd::WriteCurve as u8, 0x36);

        assert_eq!(Cmd::GetDtc as u8, 0x40);
        assert_eq!(Cmd::DtcList as u8, 0x41);
        assert_eq!(Cmd::ClearDtc as u8, 0x42);
        assert_eq!(Cmd::GetSensorRaw as u8, 0x43);
        assert_eq!(Cmd::SensorRaw as u8, 0x44);
        assert_eq!(Cmd::RunOutputTest as u8, 0x45);
        assert_eq!(Cmd::GetOutputTestDirectory as u8, 0x46);
        assert_eq!(Cmd::OutputTestDirectory as u8, 0x47);
        assert_eq!(Cmd::GetSensorRawDirectory as u8, 0x48);
        assert_eq!(Cmd::SensorRawDirectory as u8, 0x49);
        assert_eq!(Cmd::GetFreezeFrames as u8, 0x4A);
        assert_eq!(Cmd::FreezeFrames as u8, 0x4B);
        assert_eq!(Cmd::GetTriggerCapture as u8, 0x4C);
        assert_eq!(Cmd::TriggerCapture as u8, 0x4D);
        assert_eq!(Cmd::GetTriggerDecoderDirectory as u8, 0x4E);
        assert_eq!(Cmd::TriggerDecoderDirectory as u8, 0x4F);

        assert_eq!(Cmd::EnterBootloader as u8, 0x50);
        assert_eq!(Cmd::FlashBlock as u8, 0x51);
        assert_eq!(Cmd::FlashBlockAck as u8, 0x52);
        assert_eq!(Cmd::FlashVerify as u8, 0x53);
        assert_eq!(Cmd::FlashComplete as u8, 0x54);
        assert_eq!(Cmd::GetUpdateStatus as u8, 0x55);
        assert_eq!(Cmd::UpdateStatus as u8, 0x56);
        assert_eq!(Cmd::ConfirmBootHealthy as u8, 0x57);
        assert_eq!(Cmd::FlashResume as u8, 0x58);

        assert_eq!(Cmd::LogStart as u8, 0x60);
        assert_eq!(Cmd::LogStop as u8, 0x61);
        assert_eq!(Cmd::LogStatus as u8, 0x62);
        assert_eq!(Cmd::LogStatusResponse as u8, 0x63);
        assert_eq!(Cmd::ReadLogBlock as u8, 0x64);
        assert_eq!(Cmd::LogBlockData as u8, 0x65);
        assert_eq!(Cmd::GetLogbookSummary as u8, 0x66);
        assert_eq!(Cmd::LogbookSummaryResponse as u8, 0x67);
        assert_eq!(Cmd::ResetLogbook as u8, 0x68);
        assert_eq!(Cmd::SyncRtc as u8, 0x69);

        assert_eq!(Cmd::GetPinAssignments as u8, 0x6A);
        assert_eq!(Cmd::PinAssignments as u8, 0x6B);
        assert_eq!(Cmd::PinAssign as u8, 0x6C);
        assert_eq!(Cmd::GetTriggerToothLog as u8, 0x70);
        assert_eq!(Cmd::TriggerToothLog as u8, 0x71);
        assert_eq!(Cmd::GetCanTemplateDirectory as u8, 0x72);
        assert_eq!(Cmd::CanTemplateDirectory as u8, 0x73);
        assert_eq!(Cmd::GetCanSignalDirectory as u8, 0x74);
        assert_eq!(Cmd::CanSignalDirectory as u8, 0x75);

        assert_eq!(Cmd::Ack as u8, 0xA0);
        assert_eq!(Cmd::Nack as u8, 0xA1);
        assert_eq!(Cmd::Error as u8, 0xFF);
    }

    #[test]
    fn runtime_cmd_enum_codepoints_are_unique() {
        let source = include_str!("protocol.rs");
        let enum_block = source
            .split("pub enum Cmd {")
            .nth(1)
            .and_then(|tail| tail.split('}').next())
            .expect("Cmd enum block should exist");

        let mut seen = BTreeMap::<u8, String>::new();
        for raw_line in enum_block.lines() {
            let line = raw_line.split("//").next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((name_part, value_part)) = line.split_once('=') else {
                continue;
            };
            let name = name_part.trim().trim_end_matches(',');
            let value = value_part.trim().trim_end_matches(',');
            let Some(hex) = value.strip_prefix("0x") else {
                continue;
            };
            let code = u8::from_str_radix(hex, 16).expect("hex codepoint");
            if let Some(previous) = seen.insert(code, name.to_string()) {
                panic!(
                    "duplicate Cmd codepoint 0x{code:02X}: {previous} and {name}"
                );
            }
        }

        assert!(
            seen.len() >= 55,
            "expected complete runtime Cmd coverage, got only {} entries",
            seen.len()
        );
    }

    #[test]
    fn runtime_rejects_desktop_legacy_bridge_command_ids() {
        for code in [0x10u8, 0x11, 0x12, 0x13, 0x14] {
            assert!(matches!(
                Cmd::try_from(code),
                Err(ProtocolError::UnknownCmd(v)) if v == code
            ));
        }
    }
}
