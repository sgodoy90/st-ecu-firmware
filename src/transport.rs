use crate::config::ConfigStore;
use crate::contract::{base_capabilities, Capability, FirmwareIdentity, TABLE_DIRECTORY};
use crate::diagnostics::SAMPLE_FREEZE_FRAMES;
use crate::io::{
    apply_assignment_overrides, default_pin_assignments, deserialize_assignments_from_page,
    serialize_assignments_to_page, validate_assignment_set, AssignmentError, PinAssignmentRequest,
    ResolvedPinAssignment,
};
use crate::live_data::{error, protect, status, LiveDataFrame};
use crate::network::{headless_network_profile, NetworkProfile};
use crate::protection::{ProtectionAction, ProtectionConfig, ProtectionManager, ProtectionState};
use crate::protocol::{
    decode_page_payload, decode_page_request, decode_raw_table_payload, decode_sync_rtc_payload,
    encode_ack_payload, encode_can_signal_directory_payload, encode_can_template_directory_payload,
    encode_capabilities_payload, encode_freeze_frames_payload, encode_identity_payload,
    encode_log_block_payload, encode_log_status_payload, encode_logbook_summary_payload,
    encode_nack_payload, encode_network_profile_payload, encode_output_test_directory_payload,
    encode_page_directory_payload, encode_page_payload, encode_page_statuses_payload,
    encode_pin_assignments_payload, encode_pin_directory_payload,
    encode_sensor_raw_directory_payload, encode_sensor_raw_payload, encode_table_directory_payload,
    encode_table_metadata_payload, encode_trigger_capture_payload,
    encode_trigger_decoder_directory_payload, encode_trigger_tooth_log_payload,
    CanSignalDirectoryEntry, CanTemplateDirectoryEntry, Cmd, LogStatusPayload,
    LogbookSummaryPayload, OutputTestDirectoryEntry, Packet, RawTablePayload,
    SensorRawDirectoryEntry,
};
use crate::rotational_idle::RotationalIdleRuntime;
use crate::tcu::ExternalTcuRuntime;
use crate::trigger::SUPPORTED_TRIGGER_DECODERS;
use crate::trigger_runtime::TriggerRuntime;
use crate::wideband::WidebandRuntime;
use crate::ConfigPage;
use crc::{Crc, CRC_32_ISO_HDLC};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Usb,
    Can,
    WifiBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportCapabilities {
    pub usb_supported: bool,
    pub can_supported: bool,
    pub wifi_bridge_supported: bool,
}

impl Default for TransportCapabilities {
    fn default() -> Self {
        Self {
            usb_supported: true,
            can_supported: true,
            wifi_bridge_supported: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeNackCode {
    UnsupportedCommand = 1,
    MalformedPayload = 2,
    InvalidPage = 3,
    StorageFailure = 4,
}

#[derive(Debug, Clone, Serialize)]
struct RuntimeDtcEntry {
    code: String,
    description: String,
    severity: String,
    active: bool,
    confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct FirmwareRuntime {
    pub identity: FirmwareIdentity,
    pub transport: TransportCapabilities,
    pub store: ConfigStore,
    pub capabilities: Vec<Capability>,
    pub pin_assignments: Vec<ResolvedPinAssignment>,
    pub network_profile: &'static NetworkProfile,
    pub tables: Vec<RawTablePayload>,
    dtc_entries: Vec<RuntimeDtcEntry>,
    live_sample_counter: u32,
    sync_loss_counter: u8,
    trigger_runtime: TriggerRuntime,
    tcu_runtime: ExternalTcuRuntime,
    rotational_idle_runtime: RotationalIdleRuntime,
    wideband_runtime: WidebandRuntime,
    protection_manager: ProtectionManager,
    protection_state: ProtectionState,
    flash_session_active: bool,
    flash_next_block: u32,
    flash_buffer: Vec<u8>,
    log_active: bool,
    log_storage_present: bool,
    log_rtc_synced: bool,
    logbook_entries: u8,
    log_total_sessions: u32,
    log_total_elapsed_ms: u32,
    log_total_bytes_written: u32,
    log_last_elapsed_ms: u32,
    log_last_bytes_written: u32,
    log_last_block_count: u16,
    log_last_rtc_sync_ms: u32,
    log_session_id: u32,
    log_started_sample_counter: u32,
    log_elapsed_ms_latched: u32,
    log_bytes_written: u32,
    log_block_size: u16,
    log_blocks: Vec<Vec<u8>>,
    log_staging_block: Vec<u8>,
}

const OUTPUT_TEST_DIRECTORY: [OutputTestDirectoryEntry; 10] = [
    OutputTestDirectoryEntry {
        channel: 0,
        function: "injector_1",
        label: "Injector 1",
        group: "injectors",
        default_pulse_ms: Some(5),
    },
    OutputTestDirectoryEntry {
        channel: 1,
        function: "injector_2",
        label: "Injector 2",
        group: "injectors",
        default_pulse_ms: Some(5),
    },
    OutputTestDirectoryEntry {
        channel: 8,
        function: "ignition_1",
        label: "Coil 1",
        group: "coils",
        default_pulse_ms: Some(3),
    },
    OutputTestDirectoryEntry {
        channel: 9,
        function: "ignition_2",
        label: "Coil 2",
        group: "coils",
        default_pulse_ms: Some(3),
    },
    OutputTestDirectoryEntry {
        channel: 16,
        function: "fuel_pump",
        label: "Fuel Pump",
        group: "aux",
        default_pulse_ms: None,
    },
    OutputTestDirectoryEntry {
        channel: 17,
        function: "fan_1",
        label: "Fan 1",
        group: "aux",
        default_pulse_ms: None,
    },
    OutputTestDirectoryEntry {
        channel: 19,
        function: "ac_clutch",
        label: "A/C Clutch",
        group: "aux",
        default_pulse_ms: None,
    },
    OutputTestDirectoryEntry {
        channel: 24,
        function: "idle_control",
        label: "Idle Valve",
        group: "valves",
        default_pulse_ms: None,
    },
    OutputTestDirectoryEntry {
        channel: 25,
        function: "boost_control",
        label: "Boost Solenoid 1",
        group: "valves",
        default_pulse_ms: None,
    },
    OutputTestDirectoryEntry {
        channel: 27,
        function: "vvt_b1_intake",
        label: "VVT B1 Intake",
        group: "valves",
        default_pulse_ms: None,
    },
];

const FLASH_BLOCK_HEADER_LEN: usize = 4;
const FLASH_BLOCK_MAX_BYTES: usize = 1024;
const FLASH_BUFFER_MAX_BYTES: usize = 2 * 1024 * 1024;
static FLASH_CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
const LOG_BLOCK_SIZE_BYTES: u16 = 256;
const PAGE0_MAGIC: [u8; 4] = *b"STC2";
const PAGE0_VERSION: u8 = 1;
const PAGE0_OFFSET_ENGINE_CYLINDERS: usize = 5;
const PAGE0_OFFSET_INJECTOR_FLOW_CC_MIN: usize = 26;
const PAGE0_OFFSET_DEADTIME_14V_MS: usize = 28;
const PAGE0_OFFSET_DWELL_MODE: usize = 33;
const PAGE0_OFFSET_DWELL_FIXED_MS: usize = 34;
const PAGE0_OFFSET_DWELL_MAX_MS: usize = 36;
const PAGE0_OFFSET_DWELL_MIN_MS: usize = 38;
const ACTUATOR_DEADTIME_BOUNDS_MS: (f32, f32) = (0.05, 8.0);
const ACTUATOR_DWELL_BOUNDS_MS: (f32, f32) = (0.5, 10.0);
const ACTUATOR_PULSEWIDTH_BOUNDS_MS: (f32, f32) = (0.2, 22.0);

#[derive(Debug, Clone, Copy)]
struct ActuatorRuntimeConfig {
    cylinders: u8,
    injector_flow_cc_min: f32,
    injector_deadtime_14v_ms: f32,
    dwell_mode_table: bool,
    dwell_fixed_ms: f32,
    dwell_min_ms: f32,
    dwell_max_ms: f32,
}

impl Default for ActuatorRuntimeConfig {
    fn default() -> Self {
        Self {
            cylinders: 4,
            injector_flow_cc_min: 440.0,
            injector_deadtime_14v_ms: 0.9,
            dwell_mode_table: false,
            dwell_fixed_ms: 3.2,
            dwell_min_ms: 0.8,
            dwell_max_ms: 5.0,
        }
    }
}

fn read_be_u16(payload: &[u8], offset: usize) -> Option<u16> {
    payload
        .get(offset..offset + 2)
        .map(|window| u16::from_be_bytes([window[0], window[1]]))
}

fn parse_page0_actuator_config(payload: &[u8]) -> ActuatorRuntimeConfig {
    let mut config = ActuatorRuntimeConfig::default();
    if payload.len() <= PAGE0_OFFSET_DWELL_MIN_MS + 1 {
        return config;
    }
    if payload.get(0..4) != Some(&PAGE0_MAGIC) || payload.get(4).copied() != Some(PAGE0_VERSION) {
        return config;
    }

    let cylinders = payload[PAGE0_OFFSET_ENGINE_CYLINDERS];
    config.cylinders = if cylinders == 0 { 4 } else { cylinders.min(16) };

    if let Some(flow_cc) = read_be_u16(payload, PAGE0_OFFSET_INJECTOR_FLOW_CC_MIN) {
        let flow = flow_cc as f32;
        if flow >= 50.0 {
            config.injector_flow_cc_min = flow;
        }
    }

    if let Some(deadtime_raw) = read_be_u16(payload, PAGE0_OFFSET_DEADTIME_14V_MS) {
        config.injector_deadtime_14v_ms = (deadtime_raw as f32 / 1000.0)
            .clamp(ACTUATOR_DEADTIME_BOUNDS_MS.0, ACTUATOR_DEADTIME_BOUNDS_MS.1);
    }

    config.dwell_mode_table = payload[PAGE0_OFFSET_DWELL_MODE] == 1;

    let mut dwell_fixed_ms = read_be_u16(payload, PAGE0_OFFSET_DWELL_FIXED_MS)
        .map(|value| value as f32 / 1000.0)
        .unwrap_or(config.dwell_fixed_ms)
        .clamp(ACTUATOR_DWELL_BOUNDS_MS.0, ACTUATOR_DWELL_BOUNDS_MS.1);
    let mut dwell_max_ms = read_be_u16(payload, PAGE0_OFFSET_DWELL_MAX_MS)
        .map(|value| value as f32 / 1000.0)
        .unwrap_or(config.dwell_max_ms)
        .clamp(ACTUATOR_DWELL_BOUNDS_MS.0, ACTUATOR_DWELL_BOUNDS_MS.1);
    let mut dwell_min_ms = read_be_u16(payload, PAGE0_OFFSET_DWELL_MIN_MS)
        .map(|value| value as f32 / 1000.0)
        .unwrap_or(config.dwell_min_ms)
        .clamp(ACTUATOR_DWELL_BOUNDS_MS.0, ACTUATOR_DWELL_BOUNDS_MS.1);

    if dwell_max_ms < dwell_min_ms {
        std::mem::swap(&mut dwell_max_ms, &mut dwell_min_ms);
    }
    if (dwell_max_ms - dwell_min_ms) < 0.1 {
        dwell_max_ms =
            (dwell_min_ms + 0.1).clamp(ACTUATOR_DWELL_BOUNDS_MS.0, ACTUATOR_DWELL_BOUNDS_MS.1);
        if dwell_max_ms <= dwell_min_ms {
            dwell_min_ms = (dwell_max_ms - 0.1).max(ACTUATOR_DWELL_BOUNDS_MS.0);
        }
    }
    dwell_fixed_ms = dwell_fixed_ms.clamp(dwell_min_ms, dwell_max_ms);

    config.dwell_fixed_ms = dwell_fixed_ms;
    config.dwell_min_ms = dwell_min_ms;
    config.dwell_max_ms = dwell_max_ms;

    config
}

fn injector_deadtime_for_conditions_ms(deadtime_14v_ms: f32, vbatt: f32, fuel_temp_c: f32) -> f32 {
    let voltage_comp_ms = (14.0 - vbatt).clamp(-6.0, 6.0) * 0.06;
    let temperature_scale =
        (1.0 + ((20.0 - fuel_temp_c).clamp(-60.0, 60.0) * 0.0025)).clamp(0.85, 1.2);
    ((deadtime_14v_ms + voltage_comp_ms).max(0.01) * temperature_scale)
        .clamp(ACTUATOR_DEADTIME_BOUNDS_MS.0, ACTUATOR_DEADTIME_BOUNDS_MS.1)
}

fn dwell_for_conditions_ms(
    config: &ActuatorRuntimeConfig,
    vbatt: f32,
    coolant_c: f32,
) -> (f32, f32) {
    let voltage_exponent = if config.dwell_mode_table { 0.55 } else { 0.40 };
    let voltage_scale = (14.0 / vbatt.max(8.0))
        .powf(voltage_exponent)
        .clamp(0.65, 1.7);
    let coolant_bias_ms = if coolant_c < 40.0 {
        ((40.0 - coolant_c).min(40.0)) * 0.01
    } else {
        0.0
    };

    let requested = config.dwell_fixed_ms * voltage_scale + coolant_bias_ms;
    let bounded = requested
        .clamp(config.dwell_min_ms, config.dwell_max_ms)
        .clamp(ACTUATOR_DWELL_BOUNDS_MS.0, ACTUATOR_DWELL_BOUNDS_MS.1);
    (requested, bounded)
}

fn default_dtc_entries() -> Vec<RuntimeDtcEntry> {
    vec![
        RuntimeDtcEntry {
            code: "P0113".to_string(),
            description: "Intake Air Temperature Circuit High".to_string(),
            severity: "warning".to_string(),
            active: false,
            confirmed: false,
        },
        RuntimeDtcEntry {
            code: "P0335".to_string(),
            description: "Crank Position Sensor Range".to_string(),
            severity: "critical".to_string(),
            active: true,
            confirmed: true,
        },
    ]
}

fn axis_range(count: u8, start: u16, end: u16) -> Vec<u16> {
    if count <= 1 {
        return vec![start];
    }
    let span = (end - start) as f32;
    (0..count)
        .map(|index| {
            let ratio = index as f32 / (count - 1) as f32;
            (start as f32 + ratio * span).round() as u16
        })
        .collect()
}

fn default_axis_bins(label: &str, count: u8) -> Vec<u16> {
    match label {
        "RPM" => axis_range(count, 500, 8_000),
        "MAP" => axis_range(count, 200, 2_500),
        "TPS" => axis_range(count, 0, 10_000),
        "Boost" => axis_range(count, 0, 4_000),
        _ => axis_range(count, 0, 10_000),
    }
}

fn encode_table_raw_value(signed: bool, raw: i32) -> u16 {
    if signed {
        (raw.clamp(i16::MIN as i32, i16::MAX as i32) as i16) as u16
    } else {
        raw.clamp(0, u16::MAX as i32) as u16
    }
}

fn default_tables() -> Vec<RawTablePayload> {
    TABLE_DIRECTORY
        .iter()
        .map(|entry| {
            let cell_count = entry.x_count as usize * entry.y_count as usize;
            let default = encode_table_raw_value(entry.signed, entry.default_value_raw);
            RawTablePayload {
                table_id: entry.id,
                x_count: entry.x_count,
                y_count: entry.y_count,
                x_bins: default_axis_bins(entry.x_label, entry.x_count),
                y_bins: default_axis_bins(entry.y_label, entry.y_count),
                data: vec![default; cell_count],
            }
        })
        .collect()
}

const SENSOR_RAW_DIRECTORY: [SensorRawDirectoryEntry; 11] = [
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
        channel: 1,
        key: "iat",
        label: "Intake Temp",
        unit: "degC",
        pin_id: "PC2",
        pin_label: "IAT",
        expected_min_mv: 500,
        expected_max_mv: 2800,
        fault_low_mv: 100,
        fault_high_mv: 3200,
    },
    SensorRawDirectoryEntry {
        channel: 2,
        key: "tps",
        label: "Throttle Position",
        unit: "%",
        pin_id: "PA3",
        pin_label: "TPS",
        expected_min_mv: 150,
        expected_max_mv: 3200,
        fault_low_mv: 50,
        fault_high_mv: 3250,
    },
    SensorRawDirectoryEntry {
        channel: 3,
        key: "map",
        label: "MAP Sensor",
        unit: "kPa",
        pin_id: "PC0",
        pin_label: "MAP",
        expected_min_mv: 330,
        expected_max_mv: 3000,
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
    SensorRawDirectoryEntry {
        channel: 5,
        key: "oil_pressure",
        label: "Oil Pressure",
        unit: "kPa",
        pin_id: "PC3",
        pin_label: "OILP",
        expected_min_mv: 330,
        expected_max_mv: 3000,
        fault_low_mv: 100,
        fault_high_mv: 3200,
    },
    SensorRawDirectoryEntry {
        channel: 6,
        key: "fuel_pressure",
        label: "Fuel Pressure",
        unit: "kPa",
        pin_id: "PC4",
        pin_label: "FPR",
        expected_min_mv: 330,
        expected_max_mv: 3000,
        fault_low_mv: 100,
        fault_high_mv: 3200,
    },
    SensorRawDirectoryEntry {
        channel: 7,
        key: "baro",
        label: "Barometric Pressure",
        unit: "kPa",
        pin_id: "PF8",
        pin_label: "BARO",
        expected_min_mv: 300,
        expected_max_mv: 3000,
        fault_low_mv: 100,
        fault_high_mv: 3200,
    },
    SensorRawDirectoryEntry {
        channel: 8,
        key: "wideband",
        label: "Wideband Input",
        unit: "lambda",
        pin_id: "PF9",
        pin_label: "WB_O2",
        expected_min_mv: 200,
        expected_max_mv: 3000,
        fault_low_mv: 100,
        fault_high_mv: 3200,
    },
    SensorRawDirectoryEntry {
        channel: 9,
        key: "flex_fuel",
        label: "Flex Fuel",
        unit: "%",
        pin_id: "PE9",
        pin_label: "FLEX",
        expected_min_mv: 200,
        expected_max_mv: 3000,
        fault_low_mv: 100,
        fault_high_mv: 3200,
    },
    SensorRawDirectoryEntry {
        channel: 10,
        key: "oil_temp",
        label: "Oil Temp",
        unit: "degC",
        pin_id: "PF10",
        pin_label: "OILT",
        expected_min_mv: 350,
        expected_max_mv: 2800,
        fault_low_mv: 100,
        fault_high_mv: 3200,
    },
];

const CAN_TEMPLATE_DIRECTORY: [CanTemplateDirectoryEntry; 6] = [
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
        key: "st_fault_stream_tx",
        label: "ST Fault Stream TX",
        direction: "tx",
        category: "diagnostic",
        can_id: 0x7E1,
        extended_id: false,
        dlc: 8,
    },
    CanTemplateDirectoryEntry {
        id: 3,
        key: "st_tcu_request_rx",
        label: "ST TCU Request RX",
        direction: "rx",
        category: "integration",
        can_id: 0x620,
        extended_id: false,
        dlc: 8,
    },
    CanTemplateDirectoryEntry {
        id: 4,
        key: "st_wideband_status_rx",
        label: "ST Wideband Status RX",
        direction: "rx",
        category: "wideband",
        can_id: 0x640,
        extended_id: false,
        dlc: 8,
    },
    CanTemplateDirectoryEntry {
        id: 5,
        key: "st_dash_fast_tx",
        label: "ST Dash Fast TX",
        direction: "tx",
        category: "dash",
        can_id: 0x700,
        extended_id: false,
        dlc: 8,
    },
    CanTemplateDirectoryEntry {
        id: 6,
        key: "st_dash_slow_tx",
        label: "ST Dash Slow TX",
        direction: "tx",
        category: "dash",
        can_id: 0x701,
        extended_id: false,
        dlc: 8,
    },
];

const CAN_SIGNAL_DIRECTORY: [CanSignalDirectoryEntry; 14] = [
    CanSignalDirectoryEntry {
        id: 1,
        template_id: 1,
        key: "engine_rpm",
        label: "Engine RPM",
        maps_to: "rpm",
        start_bit: 0,
        bit_length: 16,
        signed: false,
        little_endian: true,
        scale: 1.0,
        offset: 0.0,
        min: 0.0,
        max: 12_000.0,
        unit: "rpm",
    },
    CanSignalDirectoryEntry {
        id: 2,
        template_id: 1,
        key: "engine_map_kpa",
        label: "MAP",
        maps_to: "map_kpa",
        start_bit: 16,
        bit_length: 16,
        signed: false,
        little_endian: true,
        scale: 0.1,
        offset: 0.0,
        min: 0.0,
        max: 500.0,
        unit: "kPa",
    },
    CanSignalDirectoryEntry {
        id: 3,
        template_id: 1,
        key: "engine_tps_pct",
        label: "TPS",
        maps_to: "tps_pct",
        start_bit: 32,
        bit_length: 8,
        signed: false,
        little_endian: true,
        scale: 0.5,
        offset: 0.0,
        min: 0.0,
        max: 100.0,
        unit: "%",
    },
    CanSignalDirectoryEntry {
        id: 4,
        template_id: 2,
        key: "fault_active",
        label: "Fault Active",
        maps_to: "fault_active",
        start_bit: 0,
        bit_length: 1,
        signed: false,
        little_endian: true,
        scale: 1.0,
        offset: 0.0,
        min: 0.0,
        max: 1.0,
        unit: "bool",
    },
    CanSignalDirectoryEntry {
        id: 5,
        template_id: 2,
        key: "fault_code",
        label: "Fault Code",
        maps_to: "fault_code",
        start_bit: 8,
        bit_length: 16,
        signed: false,
        little_endian: true,
        scale: 1.0,
        offset: 0.0,
        min: 0.0,
        max: 65_535.0,
        unit: "raw",
    },
    CanSignalDirectoryEntry {
        id: 6,
        template_id: 3,
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
        id: 7,
        template_id: 3,
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
    CanSignalDirectoryEntry {
        id: 8,
        template_id: 3,
        key: "tcu_shift_request",
        label: "TCU Shift Request",
        maps_to: "shift_request",
        start_bit: 16,
        bit_length: 8,
        signed: false,
        little_endian: true,
        scale: 1.0,
        offset: 0.0,
        min: 0.0,
        max: 1.0,
        unit: "bool",
    },
    CanSignalDirectoryEntry {
        id: 9,
        template_id: 4,
        key: "wideband_lambda",
        label: "Wideband Lambda",
        maps_to: "lambda",
        start_bit: 0,
        bit_length: 16,
        signed: false,
        little_endian: true,
        scale: 0.0001,
        offset: 0.0,
        min: 0.6,
        max: 1.6,
        unit: "lambda",
    },
    CanSignalDirectoryEntry {
        id: 10,
        template_id: 4,
        key: "wideband_status",
        label: "Wideband Status",
        maps_to: "wideband_status",
        start_bit: 16,
        bit_length: 8,
        signed: false,
        little_endian: true,
        scale: 1.0,
        offset: 0.0,
        min: 0.0,
        max: 255.0,
        unit: "raw",
    },
    CanSignalDirectoryEntry {
        id: 11,
        template_id: 5,
        key: "dash_speed_kmh",
        label: "Dash Speed",
        maps_to: "vss_kmh",
        start_bit: 0,
        bit_length: 16,
        signed: false,
        little_endian: true,
        scale: 0.1,
        offset: 0.0,
        min: 0.0,
        max: 320.0,
        unit: "km/h",
    },
    CanSignalDirectoryEntry {
        id: 12,
        template_id: 5,
        key: "dash_gear",
        label: "Dash Gear",
        maps_to: "gear",
        start_bit: 16,
        bit_length: 8,
        signed: false,
        little_endian: true,
        scale: 1.0,
        offset: 0.0,
        min: 0.0,
        max: 10.0,
        unit: "gear",
    },
    CanSignalDirectoryEntry {
        id: 13,
        template_id: 6,
        key: "dash_coolant_c",
        label: "Dash Coolant",
        maps_to: "coolant_c",
        start_bit: 0,
        bit_length: 16,
        signed: true,
        little_endian: true,
        scale: 0.1,
        offset: 0.0,
        min: -40.0,
        max: 180.0,
        unit: "degC",
    },
    CanSignalDirectoryEntry {
        id: 14,
        template_id: 6,
        key: "dash_oil_pressure",
        label: "Dash Oil Pressure",
        maps_to: "oil_pressure_kpa",
        start_bit: 16,
        bit_length: 16,
        signed: false,
        little_endian: true,
        scale: 0.1,
        offset: 0.0,
        min: 0.0,
        max: 800.0,
        unit: "kPa",
    },
];

fn sensor_raw_sample(channel: u8) -> (u16, f32) {
    let voltage: f32 = match channel {
        0 => 2.18,
        1 => 1.94,
        2 => 0.87,
        3 => 1.42,
        4 => 2.41,
        5 => 1.26,
        6 => 1.58,
        7 => 1.52,
        8 => 2.07,
        9 => 1.74,
        10 => 1.88,
        _ => 0.0,
    };
    let adc = ((voltage / 3.3).clamp(0.0, 1.0) * 65535.0).round() as u16;
    (adc, voltage)
}

impl FirmwareRuntime {
    pub fn new(identity: FirmwareIdentity, simulator: bool) -> Self {
        let mut store = ConfigStore::new_zeroed();
        let mut trigger_runtime = TriggerRuntime::default();
        let pin_assignments = validate_assignment_set(&default_pin_assignments())
            .expect("default pin assignments must be valid for board definition");
        let page_len = store
            .page_length(ConfigPage::PinAssignment as u8)
            .expect("pin assignment page must exist");
        let pin_page = serialize_assignments_to_page(&pin_assignments, page_len)
            .expect("default pin assignments must fit pin assignment page");
        store
            .write_page(ConfigPage::PinAssignment as u8, &pin_page)
            .expect("pin assignment page seed must write");
        store
            .burn_page(ConfigPage::PinAssignment as u8)
            .expect("pin assignment page seed must burn");
        if let Some(page0) = store.read_page(ConfigPage::BaseEngineFuelComm as u8) {
            trigger_runtime.apply_page0_payload(page0);
        }
        Self {
            identity,
            transport: TransportCapabilities::default(),
            store,
            capabilities: base_capabilities(simulator),
            pin_assignments,
            network_profile: headless_network_profile(),
            tables: default_tables(),
            dtc_entries: default_dtc_entries(),
            live_sample_counter: 0,
            sync_loss_counter: 0,
            trigger_runtime,
            tcu_runtime: ExternalTcuRuntime::default(),
            rotational_idle_runtime: RotationalIdleRuntime::default(),
            wideband_runtime: WidebandRuntime::default(),
            protection_manager: ProtectionManager::new(ProtectionConfig::default()),
            protection_state: ProtectionState::default(),
            flash_session_active: false,
            flash_next_block: 0,
            flash_buffer: Vec::new(),
            log_active: false,
            log_storage_present: true,
            log_rtc_synced: false,
            logbook_entries: 0,
            log_total_sessions: 0,
            log_total_elapsed_ms: 0,
            log_total_bytes_written: 0,
            log_last_elapsed_ms: 0,
            log_last_bytes_written: 0,
            log_last_block_count: 0,
            log_last_rtc_sync_ms: 0,
            log_session_id: 0,
            log_started_sample_counter: 0,
            log_elapsed_ms_latched: 0,
            log_bytes_written: 0,
            log_block_size: LOG_BLOCK_SIZE_BYTES,
            log_blocks: Vec::new(),
            log_staging_block: Vec::new(),
        }
    }

    pub fn new_ecu_v1() -> Self {
        Self::new(FirmwareIdentity::ecu_v1(), false)
    }

    pub fn new_simulator() -> Self {
        Self::new(FirmwareIdentity::simulator(), true)
    }

    pub fn apply_pin_assignment_overrides(
        &mut self,
        overrides: &[PinAssignmentRequest<'_>],
    ) -> Result<(), AssignmentError> {
        self.pin_assignments = apply_assignment_overrides(&self.pin_assignments, overrides)?;
        self.sync_pin_assignment_page()?;
        Ok(())
    }

    pub fn pin_assignment(
        &self,
        function: crate::io::EcuFunction,
    ) -> Option<&ResolvedPinAssignment> {
        self.pin_assignments
            .iter()
            .find(|assignment| assignment.function == function)
    }

    fn sync_pin_assignment_page(&mut self) -> Result<(), AssignmentError> {
        let page_id = ConfigPage::PinAssignment as u8;
        let page_len = self
            .store
            .page_length(page_id)
            .ok_or(AssignmentError::InvalidPayload)?;
        let payload = serialize_assignments_to_page(&self.pin_assignments, page_len)?;
        self.store
            .write_page(page_id, &payload)
            .map_err(|_| AssignmentError::InvalidPayload)?;
        Ok(())
    }

    fn reload_pin_assignments_from_ram_page(&mut self) -> Result<(), AssignmentError> {
        let payload = self
            .store
            .read_page(ConfigPage::PinAssignment as u8)
            .ok_or(AssignmentError::InvalidPayload)?;
        self.pin_assignments = deserialize_assignments_from_page(payload)?;
        Ok(())
    }

    fn sync_trigger_runtime_from_page0(&mut self) {
        if let Some(payload) = self.store.read_page(ConfigPage::BaseEngineFuelComm as u8) {
            self.trigger_runtime.apply_page0_payload(payload);
        }
    }

    fn table_index(&self, table_id: u8) -> Option<usize> {
        self.tables
            .iter()
            .position(|table| table.table_id == table_id)
    }

    fn write_table_frame(&mut self, incoming: RawTablePayload) -> Result<(), RuntimeNackCode> {
        let Some(index) = self.table_index(incoming.table_id) else {
            return Err(RuntimeNackCode::MalformedPayload);
        };
        let current = &self.tables[index];
        if incoming.x_count != current.x_count
            || incoming.y_count != current.y_count
            || incoming.x_bins.len() != current.x_bins.len()
            || incoming.y_bins.len() != current.y_bins.len()
            || incoming.data.len() != current.data.len()
        {
            return Err(RuntimeNackCode::MalformedPayload);
        }

        self.tables[index] = incoming;
        Ok(())
    }

    fn current_runtime_ms(&self) -> u32 {
        self.live_sample_counter.saturating_mul(20)
    }

    fn current_log_elapsed_ms(&self) -> u32 {
        if self.log_active {
            let now_ms = self.current_runtime_ms();
            let start_ms = self.log_started_sample_counter.saturating_mul(20);
            now_ms.saturating_sub(start_ms)
        } else {
            self.log_elapsed_ms_latched
        }
    }

    fn current_log_block_count(&self) -> u16 {
        let mut count = self.log_blocks.len() as u16;
        if self.log_active && !self.log_staging_block.is_empty() {
            count = count.saturating_add(1);
        }
        count
    }

    fn append_log_bytes(&mut self, bytes: &[u8]) {
        if !self.log_active || bytes.is_empty() {
            return;
        }
        self.log_bytes_written = self.log_bytes_written.saturating_add(bytes.len() as u32);
        let block_size = self.log_block_size.max(1) as usize;
        for byte in bytes {
            self.log_staging_block.push(*byte);
            if self.log_staging_block.len() >= block_size {
                self.log_blocks
                    .push(std::mem::take(&mut self.log_staging_block));
            }
        }
        self.log_elapsed_ms_latched = self.current_log_elapsed_ms();
    }

    fn append_log_sample(&mut self, frame: &LiveDataFrame) {
        if !self.log_active {
            return;
        }
        let line = format!(
            "t={} rpm={:.0} map={:.1} tps={:.1} lambda={:.4} clt={:.1}\n",
            frame.timestamp_ms,
            frame.rpm,
            frame.map_kpa,
            frame.tps_pct,
            frame.lambda,
            frame.coolant_c
        );
        self.append_log_bytes(line.as_bytes());
    }

    fn read_log_block(&self, block_index: usize) -> Option<Vec<u8>> {
        if block_index < self.log_blocks.len() {
            return Some(self.log_blocks[block_index].clone());
        }
        if self.log_active
            && block_index == self.log_blocks.len()
            && !self.log_staging_block.is_empty()
        {
            return Some(self.log_staging_block.clone());
        }
        None
    }

    fn build_logbook_summary(&self) -> LogbookSummaryPayload {
        LogbookSummaryPayload {
            sessions: self.log_total_sessions,
            entries: self.logbook_entries as u16,
            total_elapsed_ms: self.log_total_elapsed_ms,
            total_bytes_written: self.log_total_bytes_written,
            last_session_id: self.log_session_id,
            last_elapsed_ms: self.log_last_elapsed_ms,
            last_bytes_written: self.log_last_bytes_written,
            last_block_count: self.log_last_block_count,
            rtc_synced: self.log_rtc_synced,
            last_rtc_sync_ms: self.log_last_rtc_sync_ms,
        }
    }

    fn reset_logbook(&mut self) {
        self.logbook_entries = 0;
        self.log_total_sessions = 0;
        self.log_total_elapsed_ms = 0;
        self.log_total_bytes_written = 0;
        self.log_last_elapsed_ms = 0;
        self.log_last_bytes_written = 0;
        self.log_last_block_count = 0;
        self.log_session_id = 0;
        self.log_elapsed_ms_latched = 0;
        self.log_bytes_written = 0;
        self.log_blocks.clear();
        self.log_staging_block.clear();
    }

    fn build_live_data_frame(&mut self) -> LiveDataFrame {
        self.live_sample_counter = self.live_sample_counter.wrapping_add(1);
        self.trigger_runtime.observe_tick(self.live_sample_counter);
        if self.live_sample_counter % 190 == 0 {
            self.sync_loss_counter = self.sync_loss_counter.wrapping_add(1);
        }

        let mut frame = LiveDataFrame::default();
        frame.timestamp_ms = self.current_runtime_ms();
        let tcu = self.tcu_runtime.tick(self.live_sample_counter);
        let shift_in_progress = self.tcu_runtime.shift_in_progress();
        let actuator_config = self
            .store
            .read_page(ConfigPage::BaseEngineFuelComm as u8)
            .map(parse_page0_actuator_config)
            .unwrap_or_default();

        frame.sync_loss_counter = self.sync_loss_counter;
        frame.rpm = if shift_in_progress { 2_150.0 } else { 980.0 };
        frame.vss_kmh = if shift_in_progress { 46.0 } else { 44.0 };
        frame.tps_pct = if shift_in_progress { 24.0 } else { 3.5 };
        frame.map_kpa = if shift_in_progress { 62.0 } else { 36.0 };
        frame.baro_kpa = 100.4;
        frame.boost_kpa = frame.map_kpa - frame.baro_kpa;
        frame.oil_pressure_kpa = if shift_in_progress { 255.0 } else { 298.0 };
        frame.fuel_pressure_kpa = 392.0;
        frame.coolant_c = 88.5;
        frame.intake_c = 43.0;
        frame.oil_temp_c = 92.0;
        frame.fuel_temp_c = 37.0;
        frame.aux_temp1_c = 805.0 + ((self.live_sample_counter % 8) as f32);
        frame.aux_temp2_c = 790.0 + ((self.live_sample_counter % 6) as f32);
        frame.afr_target = 14.7;
        frame.vbatt =
            (13.8 + ((self.live_sample_counter % 11) as f32 * 0.03) - 0.12).clamp(11.5, 15.2);
        frame.vref_mv = 5.0;
        frame.advance_deg = if shift_in_progress { 8.0 } else { 13.5 };
        frame.fuel_load = ((frame.map_kpa / frame.baro_kpa.max(1.0)) * 100.0).clamp(10.0, 250.0);
        frame.ign_load = frame.fuel_load;
        frame.ve_pct = (52.0 + frame.fuel_load * 0.21).clamp(35.0, 120.0);
        frame.idle_target_rpm = 980;
        frame.idle_valve_pct = 34;
        frame.status_flags =
            status::RUNNING | status::CAN_ACTIVE | status::USB_CONNECTED | status::CLOSED_LOOP;
        if shift_in_progress {
            frame.status_flags |= status::FLAT_SHIFT;
        }
        frame.correction_iat = ((frame.intake_c - 25.0) * -0.12).clamp(-8.0, 8.0);
        frame.correction_clt = if frame.coolant_c < 80.0 {
            ((80.0 - frame.coolant_c) * 0.12).clamp(0.0, 8.0)
        } else {
            0.0
        };
        frame.correction_baro = ((frame.baro_kpa - 101.3) * 0.08).clamp(-5.0, 5.0);
        frame.correction_flex = 0.0;

        let flow_scale = (460.0 / actuator_config.injector_flow_cc_min.max(50.0)).clamp(0.35, 2.4);
        let base_pulsewidth_ms = (if shift_in_progress {
            2.9
        } else {
            1.65 + frame.tps_pct * 0.011 + (frame.map_kpa - 30.0).max(0.0) * 0.006
        }) * flow_scale;
        let deadtime_ms = injector_deadtime_for_conditions_ms(
            actuator_config.injector_deadtime_14v_ms,
            frame.vbatt,
            frame.fuel_temp_c,
        );
        let requested_pulsewidth_ms = base_pulsewidth_ms + deadtime_ms;
        frame.actual_pulsewidth_ms = requested_pulsewidth_ms.clamp(
            ACTUATOR_PULSEWIDTH_BOUNDS_MS.0,
            ACTUATOR_PULSEWIDTH_BOUNDS_MS.1,
        );
        if (frame.actual_pulsewidth_ms - requested_pulsewidth_ms).abs() > 0.001 {
            frame.error_flags |= error::INJECTOR;
            frame.status_flags |= status::CHECK_ENGINE;
        }
        let event_cycle_ms = (120_000.0 / frame.rpm.max(1.0)).max(1.0);
        frame.injector_duty_pct =
            ((frame.actual_pulsewidth_ms / event_cycle_ms) * 100.0).clamp(0.0, 99.5);
        if frame.injector_duty_pct > 95.0 {
            frame.error_flags |= error::INJECTOR;
            frame.status_flags |= status::CHECK_ENGINE;
        }
        let (requested_dwell_ms, bounded_dwell_ms) =
            dwell_for_conditions_ms(&actuator_config, frame.vbatt, frame.coolant_c);
        frame.dwell_ms = bounded_dwell_ms;
        if (bounded_dwell_ms - requested_dwell_ms).abs() > 0.001 {
            frame.error_flags |= error::IGNITION;
            frame.status_flags |= status::CHECK_ENGINE;
        }

        let wideband = self
            .wideband_runtime
            .tick(self.live_sample_counter, frame.rpm > 450.0);
        frame.lambda = wideband.lambda_primary;
        frame.lambda2 = wideband.lambda_secondary;
        if wideband.heater_ready {
            frame.status_flags |= status::WIDEBAND_HEATER_READY;
        }
        if wideband.integrated_active {
            frame.status_flags |= status::WIDEBAND_INTEGRATED_ACTIVE;
        }
        if wideband.analog_fallback {
            frame.status_flags |= status::WIDEBAND_ANALOG_FALLBACK;
        }
        if matches!(
            wideband.source,
            crate::wideband::WidebandSource::ExternalModule
        ) {
            frame.status_flags |= status::WIDEBAND_EXTERNAL_ACTIVE;
        }
        if wideband.primary_fault {
            frame.error_flags |= error::O2_PRIMARY;
        }
        if wideband.secondary_fault {
            frame.error_flags |= error::O2_SECONDARY;
        }
        if wideband.primary_fault || wideband.secondary_fault {
            frame.status_flags |= status::CHECK_ENGINE;
        }

        let afr_estimate = if frame.lambda.is_finite() && frame.lambda > 0.05 {
            frame.lambda * 14.7
        } else {
            frame.afr_target.max(1.0)
        };
        let protection_action = self.protection_manager.evaluate(
            &mut self.protection_state,
            frame.rpm,
            frame.map_kpa,
            frame.oil_pressure_kpa,
            frame.coolant_c,
            afr_estimate,
            frame.aux_temp1_c,
            frame.aux_temp2_c,
        );
        frame.protect_flags = 0;
        if self.protection_state.rpm_protect {
            frame.protect_flags |= protect::RPM;
        }
        if self.protection_state.map_protect {
            frame.protect_flags |= protect::MAP;
        }
        if self.protection_state.oil_protect {
            frame.protect_flags |= protect::OIL;
        }
        if self.protection_state.afr_protect {
            frame.protect_flags |= protect::AFR;
        }
        if self.protection_state.coolant_protect {
            frame.protect_flags |= protect::COOLANT;
        }
        if !matches!(protection_action, ProtectionAction::None) {
            frame.status_flags |= status::CHECK_ENGINE;
        }
        if self.protection_state.fuel_enrich_factor > 1.0 {
            frame.fuel_correction_pct =
                ((self.protection_state.fuel_enrich_factor - 1.0) * 100.0).clamp(0.0, 100.0);
        }
        match protection_action {
            ProtectionAction::None => {}
            ProtectionAction::IgnitionRetard => {
                frame.advance_deg = frame.advance_deg.min(-8.0);
            }
            ProtectionAction::SparkCut => {
                frame.status_flags |= status::BOOST_CUT_SPARK | status::OVERREV;
            }
            ProtectionAction::FuelEnrich => {
                frame.fuel_correction_pct = frame.fuel_correction_pct.max(5.0);
            }
            ProtectionAction::FuelCut => {
                frame.status_flags |= status::BOOST_CUT_FUEL;
                frame.actual_pulsewidth_ms = 0.0;
                frame.injector_duty_pct = 0.0;
            }
            ProtectionAction::SparkAndFuelCut => {
                frame.status_flags |= status::BOOST_CUT_SPARK | status::BOOST_CUT_FUEL;
                frame.status_flags |= status::OVERREV;
                frame.actual_pulsewidth_ms = 0.0;
                frame.injector_duty_pct = 0.0;
                frame.dwell_ms = 0.0;
            }
            ProtectionAction::LimpMode => {
                frame.status_flags |= status::BOOST_CUT_SPARK | status::BOOST_CUT_FUEL;
                frame.rpm = frame.rpm.min(2_500.0);
                frame.tps_pct = frame.tps_pct.min(30.0);
                frame.actual_pulsewidth_ms = frame.actual_pulsewidth_ms.min(4.5);
                frame.injector_duty_pct = frame.injector_duty_pct.min(35.0);
                frame.dwell_ms = frame.dwell_ms.min(2.4);
            }
            ProtectionAction::Shutdown => {
                frame.status_flags |= status::BOOST_CUT_SPARK | status::BOOST_CUT_FUEL;
                frame.status_flags &= !status::RUNNING;
                frame.error_flags |= error::CRITICAL;
                frame.rpm = 0.0;
                frame.tps_pct = 0.0;
                frame.map_kpa = frame.baro_kpa;
                frame.actual_pulsewidth_ms = 0.0;
                frame.injector_duty_pct = 0.0;
                frame.dwell_ms = 0.0;
            }
        }

        let rotational = self.rotational_idle_runtime.tick(
            self.live_sample_counter,
            frame.aux_temp1_c,
            frame.tps_pct,
            frame.sync_loss_counter,
            frame.protect_flags != 0 || tcu.shift_fault_code == 5,
        );
        if rotational.active {
            frame.status_flags |= status::ROTATIONAL_IDLE_ACTIVE;
        }
        if rotational.armed {
            frame.status_flags |= status::ROTATIONAL_IDLE_ARMED;
        }
        frame.rotational_idle_cut_pct = rotational.cut_pct;
        frame.rotational_idle_timer_cs = rotational.timer_cs;
        frame.rotational_idle_active_cylinders = rotational.active_cylinders;
        frame.rotational_idle_gate_code = rotational.gate_reason_code;
        frame.rotational_idle_sync_guard_events = rotational.sync_guard_events;

        frame.transmission_status_flags = tcu.status_flags;
        frame.transmission_requested_gear = tcu.requested_gear;
        frame.transmission_torque_reduction_pct = tcu.torque_reduction_pct;
        frame.transmission_torque_reduction_timer_cs = tcu.torque_reduction_timer_cs;
        frame.transmission_shift_result_code = tcu.shift_result_code;
        frame.transmission_shift_request_counter = tcu.shift_request_counter;
        frame.transmission_shift_timeout_counter = tcu.shift_timeout_counter;
        frame.transmission_shift_fault_code = tcu.shift_fault_code;
        frame.transmission_state_code = tcu.state_code;
        frame.transmission_request_age_cs = tcu.request_age_cs;
        frame.transmission_ack_counter = tcu.ack_counter;
        self.append_log_sample(&frame);
        frame
    }

    pub fn handle_packet(&mut self, packet: Packet) -> Packet {
        match packet.cmd {
            Cmd::Ping => Packet::new(Cmd::Pong, vec![]),
            Cmd::GetVersion => Packet::new(
                Cmd::VersionResponse,
                encode_identity_payload(&self.identity, &self.capabilities),
            ),
            Cmd::GetCapabilities => Packet::new(
                Cmd::Capabilities,
                encode_capabilities_payload(&self.capabilities),
            ),
            Cmd::GetLiveData => Packet::new(
                Cmd::LiveData,
                self.build_live_data_frame().encode().to_vec(),
            ),
            Cmd::GetSensorRaw => {
                let channel = packet.payload.first().copied().unwrap_or(0);
                let (adc, voltage) = sensor_raw_sample(channel);
                Packet::new(Cmd::SensorRaw, encode_sensor_raw_payload(adc, voltage))
            }
            Cmd::GetFreezeFrames => Packet::new(
                Cmd::FreezeFrames,
                encode_freeze_frames_payload(&SAMPLE_FREEZE_FRAMES),
            ),
            Cmd::GetTriggerCapture => {
                let capture = self
                    .trigger_runtime
                    .trigger_capture(self.live_sample_counter);
                Packet::new(
                    Cmd::TriggerCapture,
                    encode_trigger_capture_payload(&capture),
                )
            }
            Cmd::GetTriggerDecoderDirectory => Packet::new(
                Cmd::TriggerDecoderDirectory,
                encode_trigger_decoder_directory_payload(&SUPPORTED_TRIGGER_DECODERS),
            ),
            Cmd::GetTriggerToothLog => {
                let tooth_log = self
                    .trigger_runtime
                    .trigger_tooth_log(self.live_sample_counter);
                Packet::new(
                    Cmd::TriggerToothLog,
                    encode_trigger_tooth_log_payload(&tooth_log),
                )
            }
            Cmd::GetCanTemplateDirectory => Packet::new(
                Cmd::CanTemplateDirectory,
                encode_can_template_directory_payload(&CAN_TEMPLATE_DIRECTORY),
            ),
            Cmd::GetCanSignalDirectory => Packet::new(
                Cmd::CanSignalDirectory,
                encode_can_signal_directory_payload(&CAN_SIGNAL_DIRECTORY),
            ),
            Cmd::LogStart => {
                self.log_active = true;
                self.log_session_id = self.log_session_id.wrapping_add(1);
                if self.log_session_id == 0 {
                    self.log_session_id = 1;
                }
                self.log_started_sample_counter = self.live_sample_counter;
                self.log_elapsed_ms_latched = 0;
                self.log_bytes_written = 0;
                self.log_blocks.clear();
                self.log_staging_block.clear();
                let header = format!(
                    "session={} start_ms={}\n",
                    self.log_session_id,
                    self.current_runtime_ms()
                );
                self.append_log_bytes(header.as_bytes());
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::LogStop => {
                if self.log_active {
                    self.log_elapsed_ms_latched = self.current_log_elapsed_ms();
                    self.log_active = false;
                    if !self.log_staging_block.is_empty() {
                        self.log_blocks
                            .push(std::mem::take(&mut self.log_staging_block));
                    }
                    self.logbook_entries = self.logbook_entries.saturating_add(1);
                    self.log_total_sessions = self.log_total_sessions.saturating_add(1);
                    self.log_total_elapsed_ms = self
                        .log_total_elapsed_ms
                        .saturating_add(self.log_elapsed_ms_latched);
                    self.log_total_bytes_written = self
                        .log_total_bytes_written
                        .saturating_add(self.log_bytes_written);
                    self.log_last_elapsed_ms = self.log_elapsed_ms_latched;
                    self.log_last_bytes_written = self.log_bytes_written;
                    self.log_last_block_count = self.current_log_block_count();
                }
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::LogStatus => Packet::new(
                Cmd::LogStatusResponse,
                encode_log_status_payload(&LogStatusPayload {
                    active: self.log_active,
                    storage_present: self.log_storage_present,
                    rtc_synced: self.log_rtc_synced,
                    logbook_entries: self.logbook_entries,
                    session_id: self.log_session_id,
                    elapsed_ms: self.current_log_elapsed_ms(),
                    bytes_written: self.log_bytes_written,
                    block_count: self.current_log_block_count(),
                    block_size: self.log_block_size,
                }),
            ),
            Cmd::GetLogbookSummary => Packet::new(
                Cmd::LogbookSummaryResponse,
                encode_logbook_summary_payload(&self.build_logbook_summary()),
            ),
            Cmd::ResetLogbook => {
                if self.log_active {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "cannot reset logbook while logging",
                    );
                }
                self.reset_logbook();
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::SyncRtc => match decode_sync_rtc_payload(&packet.payload) {
                Ok(epoch_ms) => {
                    self.log_rtc_synced = true;
                    self.log_last_rtc_sync_ms = epoch_ms.min(u32::MAX as u64) as u32;
                    Packet::new(Cmd::Ack, vec![])
                }
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad sync-rtc payload"),
            },
            Cmd::ReadLogBlock => {
                if packet.payload.len() != 2 {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "bad read-log-block payload",
                    );
                }
                let block_index = u16::from_be_bytes([packet.payload[0], packet.payload[1]]);
                let total_blocks = self.current_log_block_count();
                if block_index >= total_blocks {
                    return nack(RuntimeNackCode::MalformedPayload, "invalid log block index");
                }
                let Some(block_data) = self.read_log_block(block_index as usize) else {
                    return nack(RuntimeNackCode::MalformedPayload, "log block unavailable");
                };
                Packet::new(
                    Cmd::LogBlockData,
                    encode_log_block_payload(block_index, total_blocks, &block_data),
                )
            }
            Cmd::GetPageDirectory => {
                Packet::new(Cmd::PageDirectory, encode_page_directory_payload())
            }
            Cmd::GetTableDirectory => {
                Packet::new(Cmd::TableDirectory, encode_table_directory_payload())
            }
            Cmd::GetTableMetadata => {
                Packet::new(Cmd::TableMetadata, encode_table_metadata_payload())
            }
            Cmd::ReadTable => {
                let Some(&table_id) = packet.payload.first() else {
                    return nack(RuntimeNackCode::MalformedPayload, "bad read-table payload");
                };
                match self.tables.iter().find(|table| table.table_id == table_id) {
                    Some(table) => Packet::new(Cmd::TableData, table.to_payload()),
                    None => nack(RuntimeNackCode::MalformedPayload, "invalid table id"),
                }
            }
            Cmd::WriteTable => match decode_raw_table_payload(&packet.payload) {
                Ok(table) => match self.write_table_frame(table) {
                    Ok(()) => Packet::new(Cmd::Ack, vec![]),
                    Err(_) => nack(RuntimeNackCode::MalformedPayload, "invalid table payload"),
                },
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad write-table payload"),
            },
            Cmd::WriteCell => {
                if packet.payload.len() != 5 {
                    return nack(RuntimeNackCode::MalformedPayload, "bad write-cell payload");
                }
                let table_id = packet.payload[0];
                let x_index = packet.payload[1] as usize;
                let y_index = packet.payload[2] as usize;
                let value = u16::from_be_bytes([packet.payload[3], packet.payload[4]]);
                let Some(index) = self.table_index(table_id) else {
                    return nack(RuntimeNackCode::MalformedPayload, "invalid table id");
                };
                let table = &mut self.tables[index];
                if x_index >= table.x_count as usize || y_index >= table.y_count as usize {
                    return nack(RuntimeNackCode::MalformedPayload, "cell out of bounds");
                }
                let offset = y_index * table.x_count as usize + x_index;
                table.data[offset] = value;
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::GetPinDirectory => Packet::new(Cmd::PinDirectory, encode_pin_directory_payload()),
            Cmd::GetPinAssignments => Packet::new(
                Cmd::PinAssignments,
                encode_pin_assignments_payload(&self.pin_assignments),
            ),
            Cmd::GetDtc => {
                let payload =
                    serde_json::to_vec(&self.dtc_entries).unwrap_or_else(|_| b"[]".to_vec());
                Packet::new(Cmd::DtcList, payload)
            }
            Cmd::ClearDtc => {
                self.dtc_entries.clear();
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::GetOutputTestDirectory => Packet::new(
                Cmd::OutputTestDirectory,
                encode_output_test_directory_payload(&OUTPUT_TEST_DIRECTORY),
            ),
            Cmd::RunOutputTest => {
                if packet.payload.len() < 2 {
                    return nack(RuntimeNackCode::MalformedPayload, "bad output-test payload");
                }
                if self.flash_session_active {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "output-test blocked during flash session",
                    );
                }
                let protection_active = self.protection_state.rpm_protect
                    || self.protection_state.map_protect
                    || self.protection_state.oil_protect
                    || self.protection_state.afr_protect
                    || self.protection_state.coolant_protect
                    || self.protection_state.egt_protect;
                if protection_active {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "output-test blocked by active protection",
                    );
                }
                let channel = packet.payload[0];
                let known_channel = OUTPUT_TEST_DIRECTORY
                    .iter()
                    .any(|entry| entry.channel == channel);
                if !known_channel {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "unknown output-test channel",
                    );
                }
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::GetSensorRawDirectory => Packet::new(
                Cmd::SensorRawDirectory,
                encode_sensor_raw_directory_payload(&SENSOR_RAW_DIRECTORY),
            ),
            Cmd::GetPageStatuses => Packet::new(
                Cmd::PageStatuses,
                encode_page_statuses_payload(&self.store.all_page_statuses()),
            ),
            Cmd::GetNetworkProfile => Packet::new(
                Cmd::NetworkProfile,
                encode_network_profile_payload(self.network_profile),
            ),
            Cmd::ReadPage => match decode_page_request(&packet.payload) {
                Ok(page_id) => match self.store.read_page(page_id) {
                    Some(page) => Packet::new(Cmd::PageData, encode_page_payload(page_id, page)),
                    None => nack(RuntimeNackCode::InvalidPage, "invalid page id"),
                },
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad read-page payload"),
            },
            Cmd::WritePage => match decode_page_payload(&packet.payload) {
                Ok(page) => {
                    let previous_assignments = self.pin_assignments.clone();
                    let write_result = self.store.write_page(page.page_id, &page.payload);
                    if write_result.is_err() {
                        return nack(RuntimeNackCode::StorageFailure, "page write failed");
                    }

                    if page.page_id == ConfigPage::PinAssignment as u8
                        && self.reload_pin_assignments_from_ram_page().is_err()
                    {
                        let _ = self.store.write_page(
                            ConfigPage::PinAssignment as u8,
                            &serialize_assignments_to_page(
                                &previous_assignments,
                                self.store
                                    .page_length(ConfigPage::PinAssignment as u8)
                                    .unwrap_or(page.payload.len()),
                            )
                            .unwrap_or_else(|_| vec![0u8; page.payload.len()]),
                        );
                        self.pin_assignments = previous_assignments;
                        return nack(
                            RuntimeNackCode::MalformedPayload,
                            "invalid pin-assignment page payload",
                        );
                    }

                    if page.page_id == ConfigPage::BaseEngineFuelComm as u8 {
                        self.sync_trigger_runtime_from_page0();
                    }

                    Packet::new(
                        Cmd::Ack,
                        encode_ack_payload(
                            page.page_id,
                            self.store.needs_burn(page.page_id).unwrap_or(false),
                        ),
                    )
                }
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad write-page payload"),
            },
            Cmd::BurnPage => match decode_page_request(&packet.payload) {
                Ok(page_id) => match self.store.burn_page(page_id) {
                    Ok(()) => {
                        if page_id == ConfigPage::BaseEngineFuelComm as u8 {
                            self.sync_trigger_runtime_from_page0();
                        }
                        Packet::new(Cmd::Ack, encode_ack_payload(page_id, false))
                    }
                    Err(_) => nack(RuntimeNackCode::StorageFailure, "page burn failed"),
                },
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad burn-page payload"),
            },
            Cmd::EnterBootloader => {
                self.flash_session_active = true;
                self.flash_next_block = 0;
                self.flash_buffer.clear();
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::FlashBlock => {
                if !self.flash_session_active {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "flash session not started",
                    );
                }
                if packet.payload.len() <= FLASH_BLOCK_HEADER_LEN {
                    return nack(RuntimeNackCode::MalformedPayload, "bad flash-block payload");
                }
                if packet.payload.len() > FLASH_BLOCK_HEADER_LEN + FLASH_BLOCK_MAX_BYTES {
                    return nack(RuntimeNackCode::MalformedPayload, "flash block too large");
                }
                let block_index = u32::from_be_bytes([
                    packet.payload[0],
                    packet.payload[1],
                    packet.payload[2],
                    packet.payload[3],
                ]);
                if block_index != self.flash_next_block {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "unexpected flash block index",
                    );
                }
                let payload = &packet.payload[FLASH_BLOCK_HEADER_LEN..];
                if self.flash_buffer.len().saturating_add(payload.len()) > FLASH_BUFFER_MAX_BYTES {
                    return nack(RuntimeNackCode::MalformedPayload, "flash image too large");
                }
                self.flash_buffer.extend_from_slice(payload);
                self.flash_next_block = self.flash_next_block.saturating_add(1);
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::FlashVerify => {
                if !self.flash_session_active || self.flash_buffer.is_empty() {
                    return nack(RuntimeNackCode::MalformedPayload, "nothing to verify");
                }
                if !packet.payload.is_empty() && packet.payload.len() != 4 {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "bad flash-verify payload",
                    );
                }
                if self.flash_buffer.iter().all(|byte| *byte == 0) {
                    return nack(RuntimeNackCode::MalformedPayload, "flash image is blank");
                }
                if packet.payload.len() == 4 {
                    let expected_crc = u32::from_be_bytes([
                        packet.payload[0],
                        packet.payload[1],
                        packet.payload[2],
                        packet.payload[3],
                    ]);
                    let actual_crc = FLASH_CRC32.checksum(&self.flash_buffer);
                    if expected_crc != actual_crc {
                        return nack(RuntimeNackCode::MalformedPayload, "flash crc mismatch");
                    }
                }
                Packet::new(Cmd::Ack, vec![])
            }
            Cmd::FlashComplete => {
                if !self.flash_session_active {
                    return nack(
                        RuntimeNackCode::MalformedPayload,
                        "flash session not started",
                    );
                }
                self.flash_session_active = false;
                self.flash_next_block = 0;
                self.flash_buffer.clear();
                Packet::new(Cmd::Ack, vec![])
            }
            _ => nack(
                RuntimeNackCode::UnsupportedCommand,
                "command not implemented",
            ),
        }
    }
}

fn nack(code: RuntimeNackCode, reason: &str) -> Packet {
    Packet::new(Cmd::Nack, encode_nack_payload(code as u8, reason))
}

#[cfg(test)]
mod tests {
    use crate::contract::Capability;
    use crate::io::{
        apply_assignment_overrides, deserialize_assignments_from_page,
        serialize_assignments_to_page, EcuFunction, PinAssignmentRequest,
    };
    use crate::live_data::{
        error, protect, status, transmission_status, LiveDataFrame, LIVE_DATA_SIZE,
    };
    use crate::network::{MessageClass, ProductTrack, TransportLinkKind};
    use crate::protocol::{
        decode_ack_payload, decode_can_signal_directory_payload,
        decode_can_template_directory_payload, decode_capabilities_payload,
        decode_freeze_frames_payload, decode_identity_payload, decode_log_block_payload,
        decode_log_status_payload, decode_logbook_summary_payload, decode_nack_payload,
        decode_network_profile_payload, decode_output_test_directory_payload, decode_page_payload,
        decode_page_statuses_payload, decode_pin_assignments_payload, decode_pin_directory_payload,
        decode_raw_table_payload, decode_sensor_raw_directory_payload, decode_sensor_raw_payload,
        decode_trigger_capture_payload, decode_trigger_decoder_directory_payload,
        decode_trigger_tooth_log_payload, encode_page_payload, encode_page_request,
        encode_sync_rtc_payload, Cmd, Packet,
    };
    use crate::ConfigPage;
    use serde_json::Value;

    use super::{
        parse_page0_actuator_config, FirmwareRuntime, ACTUATOR_DWELL_BOUNDS_MS,
        ACTUATOR_PULSEWIDTH_BOUNDS_MS,
    };

    fn decode_live_frame(payload: &[u8]) -> LiveDataFrame {
        assert_eq!(payload.len(), LIVE_DATA_SIZE);
        let mut bytes = [0u8; LIVE_DATA_SIZE];
        bytes.copy_from_slice(payload);
        LiveDataFrame::decode(&bytes)
    }

    #[test]
    fn get_version_returns_identity_payload() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let response = runtime.handle_packet(Packet::new(Cmd::GetVersion, vec![]));
        let decoded = decode_identity_payload(&response.payload).unwrap();

        assert_eq!(response.cmd, Cmd::VersionResponse);
        assert_eq!(decoded.board_id, "st-ecu-v1");
    }

    #[test]
    fn get_capabilities_returns_capability_list() {
        let mut runtime = FirmwareRuntime::new_simulator();
        let response = runtime.handle_packet(Packet::new(Cmd::GetCapabilities, vec![]));
        let capabilities = decode_capabilities_payload(&response.payload).unwrap();

        assert_eq!(response.cmd, Cmd::Capabilities);
        assert!(!capabilities.is_empty());
    }

    #[test]
    fn can_directory_handlers_return_runtime_catalog_payloads() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let templates = runtime.handle_packet(Packet::new(Cmd::GetCanTemplateDirectory, vec![]));
        assert_eq!(templates.cmd, Cmd::CanTemplateDirectory);
        let decoded_templates = decode_can_template_directory_payload(&templates.payload).unwrap();
        assert!(decoded_templates
            .iter()
            .any(|entry| entry.key == "st_engine_stream_tx"));
        assert!(decoded_templates
            .iter()
            .any(|entry| entry.direction == "rx"));

        let signals = runtime.handle_packet(Packet::new(Cmd::GetCanSignalDirectory, vec![]));
        assert_eq!(signals.cmd, Cmd::CanSignalDirectory);
        let decoded_signals = decode_can_signal_directory_payload(&signals.payload).unwrap();
        assert!(decoded_signals.iter().any(|entry| entry.maps_to == "rpm"));
        assert!(decoded_signals
            .iter()
            .any(|entry| entry.maps_to == "torque_reduction_pct"));
    }

    #[test]
    fn page0_actuator_parser_sanitizes_extreme_values() {
        let mut payload = vec![0u8; 512];
        payload[0..4].copy_from_slice(b"STC2");
        payload[4] = 1;
        payload[5] = 0; // cylinder count zero should fall back to sane value
        payload[26..28].copy_from_slice(&20u16.to_be_bytes()); // unrealistically small injector flow
        payload[28..30].copy_from_slice(&40_000u16.to_be_bytes()); // deadtime too high
        payload[33] = 1; // dwell table mode
        payload[34..36].copy_from_slice(&12_000u16.to_be_bytes()); // fixed dwell too high
        payload[36..38].copy_from_slice(&300u16.to_be_bytes()); // max dwell too low
        payload[38..40].copy_from_slice(&9_500u16.to_be_bytes()); // min dwell > max dwell

        let parsed = parse_page0_actuator_config(&payload);
        assert_eq!(parsed.cylinders, 4);
        assert!(parsed.injector_flow_cc_min >= 100.0);
        assert!((0.05..=8.0).contains(&parsed.injector_deadtime_14v_ms));
        assert!(parsed.dwell_mode_table);
        assert!(parsed.dwell_max_ms >= parsed.dwell_min_ms);
        assert!(parsed.dwell_fixed_ms >= parsed.dwell_min_ms);
        assert!(parsed.dwell_fixed_ms <= parsed.dwell_max_ms);
    }

    #[test]
    fn runtime_live_data_binds_pulsewidth_and_dwell_to_guardrails() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let page_id = ConfigPage::BaseEngineFuelComm as u8;
        let mut page = vec![0u8; runtime.store.page_length(page_id).unwrap()];
        page[0..4].copy_from_slice(b"STC2");
        page[4] = 1;
        page[5] = 4;
        page[26..28].copy_from_slice(&100u16.to_be_bytes());
        page[28..30].copy_from_slice(&40_000u16.to_be_bytes());
        page[33] = 0;
        page[34..36].copy_from_slice(&11_000u16.to_be_bytes());
        page[36..38].copy_from_slice(&11_000u16.to_be_bytes());
        page[38..40].copy_from_slice(&200u16.to_be_bytes());

        let write = runtime.handle_packet(Packet::new(
            Cmd::WritePage,
            encode_page_payload(page_id, &page),
        ));
        assert_eq!(write.cmd, Cmd::Ack);

        let response = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
        let frame = decode_live_frame(&response.payload);
        assert!(frame.actual_pulsewidth_ms >= ACTUATOR_PULSEWIDTH_BOUNDS_MS.0);
        assert!(frame.actual_pulsewidth_ms <= ACTUATOR_PULSEWIDTH_BOUNDS_MS.1);
        assert!(frame.dwell_ms >= ACTUATOR_DWELL_BOUNDS_MS.0);
        assert!(frame.dwell_ms <= ACTUATOR_DWELL_BOUNDS_MS.1);
        assert!(frame.injector_duty_pct <= 99.5);
    }

    #[test]
    fn runtime_live_data_tcu_extensions_progress_over_time() {
        let mut runtime = FirmwareRuntime::new_simulator();
        let mut saw_shift_state = false;
        let mut saw_request_age = false;
        let mut saw_online = false;
        let mut saw_offline = false;
        let mut max_request_counter = 0u8;
        let mut max_ack_counter = 0u8;

        for _ in 0..90 {
            let response = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
            assert_eq!(response.cmd, Cmd::LiveData);
            let frame = decode_live_frame(&response.payload);
            let link_online =
                (frame.transmission_status_flags & transmission_status::TCU_LINK_ONLINE) != 0;
            saw_online |= link_online;
            saw_offline |= !link_online;
            saw_shift_state |=
                frame.transmission_state_code == 1 || frame.transmission_state_code == 3;
            saw_request_age |= frame.transmission_request_age_cs > 0;
            max_request_counter = max_request_counter.max(frame.transmission_shift_request_counter);
            max_ack_counter = max_ack_counter.max(frame.transmission_ack_counter);
        }

        assert!(saw_online, "expected link-online samples");
        assert!(saw_offline, "expected deterministic link-offline samples");
        assert!(
            saw_shift_state,
            "expected at least one request/shifting state sample"
        );
        assert!(
            saw_request_age,
            "expected request age counter to become non-zero"
        );
        assert!(
            max_request_counter > 0,
            "expected shift request counter activity"
        );
        assert!(max_ack_counter > 0, "expected shift ack counter activity");
    }

    #[test]
    fn runtime_live_data_periodically_reports_tcu_timeout_counter() {
        let mut runtime = FirmwareRuntime::new_simulator();
        let mut max_timeout_counter = 0u8;
        let mut saw_timeout_fault_code = false;

        for _ in 0..220 {
            let response = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
            let frame = decode_live_frame(&response.payload);
            max_timeout_counter = max_timeout_counter.max(frame.transmission_shift_timeout_counter);
            if frame.transmission_shift_fault_code == 2 {
                saw_timeout_fault_code = true;
            }
        }

        assert!(
            max_timeout_counter > 0,
            "expected timeout counter to increment at least once"
        );
        assert!(
            saw_timeout_fault_code,
            "expected to observe timeout fault code 2"
        );
    }

    #[test]
    fn runtime_live_data_exposes_rotational_idle_runtime_fields() {
        let mut runtime = FirmwareRuntime::new_simulator();
        let mut saw_active_cut = false;
        let mut saw_sync_guard = false;

        for _ in 0..260 {
            let response = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
            let frame = decode_live_frame(&response.payload);
            saw_active_cut |= frame.rotational_idle_cut_pct > 0
                && (frame.status_flags & status::ROTATIONAL_IDLE_ACTIVE) != 0;
            saw_sync_guard |=
                frame.rotational_idle_sync_guard_events > 0 && frame.rotational_idle_gate_code == 9;
        }

        assert!(saw_active_cut, "expected rotational-idle cut activity");
        assert!(
            saw_sync_guard,
            "expected at least one sync-guard gate event"
        );
    }

    #[test]
    fn runtime_rotational_idle_respects_protection_gate_and_sets_protect_flags() {
        let mut runtime = FirmwareRuntime::new_simulator();
        runtime.protection_manager.config.map_max_kpa.action = 30.0;

        let mut saw_map_protection = false;
        let mut saw_protection_gate = false;

        for _ in 0..80 {
            let response = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
            let frame = decode_live_frame(&response.payload);
            saw_map_protection |= (frame.protect_flags & protect::MAP) != 0;
            saw_protection_gate |= frame.rotational_idle_gate_code == 10
                && (frame.status_flags & status::ROTATIONAL_IDLE_ACTIVE) == 0;
            if saw_map_protection && saw_protection_gate {
                break;
            }
        }

        assert!(saw_map_protection, "expected MAP protection flag activity");
        assert!(
            saw_protection_gate,
            "expected rotational-idle protection gate reason (10)"
        );
    }

    #[test]
    fn runtime_rotational_idle_tracks_diagnostics_counters() {
        let mut runtime = FirmwareRuntime::new_simulator();

        for _ in 0..720 {
            let _ = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
        }
        for tick in 0..620 {
            let _ = runtime
                .rotational_idle_runtime
                .tick(tick, 860.0, 2.0, 0, false);
        }

        let diagnostics = runtime.rotational_idle_runtime.diagnostics();
        assert!(
            diagnostics.activation_count > 0,
            "expected rotational-idle activation counter"
        );
        assert!(
            diagnostics.sync_guard_events_total > 0,
            "expected sync-guard event accounting"
        );
        assert!(
            diagnostics.timeout_count > 0,
            "expected timeout-event accounting"
        );
        assert!(
            diagnostics.gate_reason_histogram[9] > 0,
            "expected sync-guard gate histogram activity"
        );
        assert!(
            diagnostics.gate_reason_histogram[8] > 0,
            "expected timeout gate histogram activity"
        );
    }

    #[test]
    fn runtime_live_data_exposes_wideband_status_and_fault_windows() {
        let mut runtime = FirmwareRuntime::new_simulator();
        let mut saw_heater_ready = false;
        let mut saw_integrated_mode = false;
        let mut saw_analog_fallback = false;
        let mut saw_external_module = false;
        let mut saw_primary_fault = false;

        for _ in 0..320 {
            let response = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
            let frame = decode_live_frame(&response.payload);
            saw_heater_ready |= (frame.status_flags & status::WIDEBAND_HEATER_READY) != 0;
            saw_integrated_mode |= (frame.status_flags & status::WIDEBAND_INTEGRATED_ACTIVE) != 0;
            saw_analog_fallback |= (frame.status_flags & status::WIDEBAND_ANALOG_FALLBACK) != 0;
            saw_external_module |= (frame.status_flags & status::WIDEBAND_EXTERNAL_ACTIVE) != 0;
            saw_primary_fault |= (frame.error_flags & error::O2_PRIMARY) != 0;
        }

        assert!(saw_heater_ready, "expected heater-ready status bit");
        assert!(saw_integrated_mode, "expected integrated wideband mode bit");
        assert!(saw_analog_fallback, "expected fallback status bit");
        assert!(saw_external_module, "expected external wideband mode bit");
        assert!(
            saw_primary_fault,
            "expected deterministic primary O2 fault window"
        );
    }

    #[test]
    fn write_read_and_burn_page_flow_is_consistent() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let data = vec![0xCC; runtime.store.page_length(0).unwrap()];

        let write =
            runtime.handle_packet(Packet::new(Cmd::WritePage, encode_page_payload(0, &data)));
        assert_eq!(write.cmd, Cmd::Ack);
        assert_eq!(decode_ack_payload(&write.payload).unwrap(), (0, true));

        let read = runtime.handle_packet(Packet::new(Cmd::ReadPage, encode_page_request(0)));
        let decoded_page = decode_page_payload(&read.payload).unwrap();
        assert_eq!(read.cmd, Cmd::PageData);
        assert_eq!(decoded_page.payload, data);

        let burn = runtime.handle_packet(Packet::new(Cmd::BurnPage, encode_page_request(0)));
        assert_eq!(burn.cmd, Cmd::Ack);
        assert_eq!(decode_ack_payload(&burn.payload).unwrap(), (0, false));
    }

    #[test]
    fn write_read_and_burn_page_flow_is_consistent_for_pages_2_5_9() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        for page_id in [2u8, 5u8, 9u8] {
            let len = runtime.store.page_length(page_id).unwrap();
            let data = (0..len)
                .map(|index| page_id.wrapping_mul(29).wrapping_add((index % 251) as u8))
                .collect::<Vec<_>>();

            let write = runtime.handle_packet(Packet::new(
                Cmd::WritePage,
                encode_page_payload(page_id, &data),
            ));
            assert_eq!(write.cmd, Cmd::Ack);
            assert_eq!(decode_ack_payload(&write.payload).unwrap(), (page_id, true));

            let read =
                runtime.handle_packet(Packet::new(Cmd::ReadPage, encode_page_request(page_id)));
            let decoded_page = decode_page_payload(&read.payload).unwrap();
            assert_eq!(read.cmd, Cmd::PageData);
            assert_eq!(decoded_page.payload, data);

            let statuses_before = runtime.handle_packet(Packet::new(Cmd::GetPageStatuses, vec![]));
            let decoded_before = decode_page_statuses_payload(&statuses_before.payload).unwrap();
            assert!(
                decoded_before
                    .iter()
                    .any(|status| status.page_id == page_id && status.needs_burn),
                "page {page_id} should require burn after write"
            );

            let burn =
                runtime.handle_packet(Packet::new(Cmd::BurnPage, encode_page_request(page_id)));
            assert_eq!(burn.cmd, Cmd::Ack);
            assert_eq!(decode_ack_payload(&burn.payload).unwrap(), (page_id, false));

            let statuses_after = runtime.handle_packet(Packet::new(Cmd::GetPageStatuses, vec![]));
            let decoded_after = decode_page_statuses_payload(&statuses_after.payload).unwrap();
            assert!(
                decoded_after
                    .iter()
                    .any(|status| status.page_id == page_id && !status.needs_burn),
                "page {page_id} should be clean after burn"
            );
        }
    }

    #[test]
    fn runtime_end_to_end_handshake_live_page_and_table_flow() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let version = runtime.handle_packet(Packet::new(Cmd::GetVersion, vec![]));
        let parsed_version = decode_identity_payload(&version.payload).unwrap();
        assert_eq!(version.cmd, Cmd::VersionResponse);
        assert_eq!(parsed_version.board_id, "st-ecu-v1");

        let capabilities = runtime.handle_packet(Packet::new(Cmd::GetCapabilities, vec![]));
        let parsed_caps = decode_capabilities_payload(&capabilities.payload).unwrap();
        assert_eq!(capabilities.cmd, Cmd::Capabilities);
        assert!(parsed_caps.contains(&Capability::LiveData));
        assert!(parsed_caps.contains(&Capability::PageWrite));
        assert!(parsed_caps.contains(&Capability::TableWrite));

        let live = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
        assert_eq!(live.cmd, Cmd::LiveData);
        let live_frame = decode_live_frame(&live.payload);
        assert!(live_frame.rpm > 0.0);

        let page_data = vec![0x5A; runtime.store.page_length(0).unwrap()];
        let write_page = runtime.handle_packet(Packet::new(
            Cmd::WritePage,
            encode_page_payload(0, &page_data),
        ));
        assert_eq!(write_page.cmd, Cmd::Ack);

        let read_page = runtime.handle_packet(Packet::new(Cmd::ReadPage, encode_page_request(0)));
        let decoded_page = decode_page_payload(&read_page.payload).unwrap();
        assert_eq!(read_page.cmd, Cmd::PageData);
        assert_eq!(decoded_page.payload, page_data);

        let burn_page = runtime.handle_packet(Packet::new(Cmd::BurnPage, encode_page_request(0)));
        assert_eq!(burn_page.cmd, Cmd::Ack);
        assert_eq!(decode_ack_payload(&burn_page.payload).unwrap(), (0, false));

        let baseline = runtime.handle_packet(Packet::new(Cmd::ReadTable, vec![0x03]));
        let mut decoded_table = decode_raw_table_payload(&baseline.payload).unwrap();
        decoded_table.data[0] = decoded_table.data[0].wrapping_add(3);

        let write_table =
            runtime.handle_packet(Packet::new(Cmd::WriteTable, decoded_table.to_payload()));
        assert_eq!(write_table.cmd, Cmd::Ack);

        let write_cell =
            runtime.handle_packet(Packet::new(Cmd::WriteCell, vec![0x03, 1, 2, 0x00, 0x9C]));
        assert_eq!(write_cell.cmd, Cmd::Ack);

        let table_after = runtime.handle_packet(Packet::new(Cmd::ReadTable, vec![0x03]));
        let decoded_after = decode_raw_table_payload(&table_after.payload).unwrap();
        assert_eq!(table_after.cmd, Cmd::TableData);
        assert_eq!(decoded_after.data[0], decoded_table.data[0]);
        let cell_offset = 2 * decoded_after.x_count as usize + 1;
        assert_eq!(decoded_after.data[cell_offset], 0x009C);
    }

    #[test]
    fn runtime_table_read_write_and_cell_update() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let baseline = runtime.handle_packet(Packet::new(Cmd::ReadTable, vec![0x03]));
        assert_eq!(baseline.cmd, Cmd::TableData);
        let mut decoded = decode_raw_table_payload(&baseline.payload).unwrap();
        let original_cell = decoded.data[0];

        decoded.data[0] = original_cell.wrapping_add(11);
        let write = runtime.handle_packet(Packet::new(Cmd::WriteTable, decoded.to_payload()));
        assert_eq!(write.cmd, Cmd::Ack);

        let cell_write =
            runtime.handle_packet(Packet::new(Cmd::WriteCell, vec![0x03, 2, 1, 0x12, 0x34]));
        assert_eq!(cell_write.cmd, Cmd::Ack);

        let after = runtime.handle_packet(Packet::new(Cmd::ReadTable, vec![0x03]));
        let after_decoded = decode_raw_table_payload(&after.payload).unwrap();
        assert_eq!(after_decoded.data[0], original_cell.wrapping_add(11));
        let offset = 1 * after_decoded.x_count as usize + 2;
        assert_eq!(after_decoded.data[offset], 0x1234);
    }

    #[test]
    fn runtime_reports_and_clears_dtcs() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let dtc = runtime.handle_packet(Packet::new(Cmd::GetDtc, vec![]));
        assert_eq!(dtc.cmd, Cmd::DtcList);
        let parsed: Value = serde_json::from_slice(&dtc.payload).unwrap();
        let dtcs = parsed.as_array().unwrap();
        assert!(!dtcs.is_empty());

        let clear = runtime.handle_packet(Packet::new(Cmd::ClearDtc, vec![]));
        assert_eq!(clear.cmd, Cmd::Ack);

        let dtc_after = runtime.handle_packet(Packet::new(Cmd::GetDtc, vec![]));
        let parsed_after: Value = serde_json::from_slice(&dtc_after.payload).unwrap();
        assert_eq!(parsed_after.as_array().unwrap().len(), 0);
    }

    #[test]
    fn runtime_output_test_validates_channel_directory() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let ok = runtime.handle_packet(Packet::new(Cmd::RunOutputTest, vec![25, 0x01, 0x00, 0x00]));
        assert_eq!(ok.cmd, Cmd::Ack);

        let bad =
            runtime.handle_packet(Packet::new(Cmd::RunOutputTest, vec![99, 0x01, 0x00, 0x00]));
        let (code, reason) = decode_nack_payload(&bad.payload).unwrap();
        assert_eq!(bad.cmd, Cmd::Nack);
        assert_eq!(code, 2);
        assert!(reason.contains("channel"));
    }

    #[test]
    fn runtime_flash_lifecycle_requires_ordered_blocks() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let without_session =
            runtime.handle_packet(Packet::new(Cmd::FlashBlock, vec![0, 0, 0, 0, 0xAA, 0xBB]));
        assert_eq!(without_session.cmd, Cmd::Nack);

        let start = runtime.handle_packet(Packet::new(Cmd::EnterBootloader, vec![]));
        assert_eq!(start.cmd, Cmd::Ack);

        let block0 = runtime.handle_packet(Packet::new(
            Cmd::FlashBlock,
            vec![0, 0, 0, 0, 0xAA, 0xBB, 0xCC],
        ));
        assert_eq!(block0.cmd, Cmd::Ack);

        let out_of_order =
            runtime.handle_packet(Packet::new(Cmd::FlashBlock, vec![0, 0, 0, 2, 0x01]));
        assert_eq!(out_of_order.cmd, Cmd::Nack);

        let block1 =
            runtime.handle_packet(Packet::new(Cmd::FlashBlock, vec![0, 0, 0, 1, 0xDD, 0xEE]));
        assert_eq!(block1.cmd, Cmd::Ack);

        let verify = runtime.handle_packet(Packet::new(Cmd::FlashVerify, vec![]));
        assert_eq!(verify.cmd, Cmd::Ack);
        let complete = runtime.handle_packet(Packet::new(Cmd::FlashComplete, vec![]));
        assert_eq!(complete.cmd, Cmd::Ack);
    }

    #[test]
    fn invalid_page_returns_nack() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let response = runtime.handle_packet(Packet::new(Cmd::ReadPage, encode_page_request(99)));
        let (code, reason) = decode_nack_payload(&response.payload).unwrap();

        assert_eq!(response.cmd, Cmd::Nack);
        assert_eq!(code, 3);
        assert!(reason.contains("invalid"));
    }

    #[test]
    fn runtime_starts_with_valid_default_pinout() {
        let runtime = FirmwareRuntime::new_ecu_v1();
        assert_eq!(runtime.pin_assignments.len(), 12);
        assert_eq!(
            runtime
                .pin_assignment(EcuFunction::BoostControl)
                .unwrap()
                .pin_id,
            "PB0"
        );
    }

    #[test]
    fn runtime_accepts_safe_pin_override_and_rejects_conflict() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        runtime
            .apply_pin_assignment_overrides(&[PinAssignmentRequest {
                function: EcuFunction::BoostControl,
                pin_id: "PC8",
            }])
            .unwrap();

        assert_eq!(
            runtime
                .pin_assignment(EcuFunction::BoostControl)
                .unwrap()
                .pin_id,
            "PC8"
        );

        let conflict = runtime.apply_pin_assignment_overrides(&[PinAssignmentRequest {
            function: EcuFunction::IdleControl,
            pin_id: "PB0",
        }]);
        assert!(conflict.is_err());
    }

    #[test]
    fn runtime_exposes_pin_directory_and_active_assignments() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let pins = runtime.handle_packet(Packet::new(Cmd::GetPinDirectory, vec![]));
        let decoded_pins = decode_pin_directory_payload(&pins.payload).unwrap();
        assert_eq!(pins.cmd, Cmd::PinDirectory);
        assert!(decoded_pins.iter().any(|pin| pin.pin_id == "PA0"));

        let assignments = runtime.handle_packet(Packet::new(Cmd::GetPinAssignments, vec![]));
        let decoded_assignments = decode_pin_assignments_payload(&assignments.payload).unwrap();
        assert_eq!(assignments.cmd, Cmd::PinAssignments);
        assert!(decoded_assignments
            .iter()
            .any(|assignment| assignment.function == EcuFunction::CrankInput));
    }

    #[test]
    fn runtime_exposes_output_test_directory() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let response = runtime.handle_packet(Packet::new(Cmd::GetOutputTestDirectory, vec![]));
        let entries = decode_output_test_directory_payload(&response.payload).unwrap();

        assert_eq!(response.cmd, Cmd::OutputTestDirectory);
        assert!(entries.iter().any(|entry| entry.function == "injector_1"));
        assert!(entries.iter().any(|entry| entry.group == "valves"));
    }

    #[test]
    fn runtime_exposes_sensor_raw_directory_and_samples() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let directory = runtime.handle_packet(Packet::new(Cmd::GetSensorRawDirectory, vec![]));
        let channels = decode_sensor_raw_directory_payload(&directory.payload).unwrap();
        assert_eq!(directory.cmd, Cmd::SensorRawDirectory);
        assert!(channels.iter().any(|entry| entry.key == "clt"));
        assert!(channels.iter().any(|entry| entry.pin_label == "VBATT"));
        assert!(channels.iter().any(|entry| entry.key == "wideband"));
        assert!(channels.iter().any(|entry| entry.key == "flex_fuel"));

        let sample = runtime.handle_packet(Packet::new(Cmd::GetSensorRaw, vec![4]));
        let decoded_sample = decode_sensor_raw_payload(&sample.payload).unwrap();
        assert_eq!(sample.cmd, Cmd::SensorRaw);
        assert!(decoded_sample.adc > 0);
        assert!(decoded_sample.voltage > 0.0);

        let wideband_sample = runtime.handle_packet(Packet::new(Cmd::GetSensorRaw, vec![8]));
        let decoded_wideband = decode_sensor_raw_payload(&wideband_sample.payload).unwrap();
        assert!(decoded_wideband.voltage > 0.0);
    }

    #[test]
    fn runtime_exposes_freeze_frames() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let response = runtime.handle_packet(Packet::new(Cmd::GetFreezeFrames, vec![]));
        let frames = decode_freeze_frames_payload(&response.payload).unwrap();

        assert_eq!(response.cmd, Cmd::FreezeFrames);
        assert!(frames.iter().any(|frame| frame.code == "P0118"));
        assert!(frames
            .iter()
            .any(|frame| frame.reason == "pressure_range_high"));
    }

    #[test]
    fn runtime_exposes_trigger_capture_and_decoder_directory() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let capture = runtime.handle_packet(Packet::new(Cmd::GetTriggerCapture, vec![]));
        let decoded_capture = decode_trigger_capture_payload(&capture.payload).unwrap();
        assert_eq!(capture.cmd, Cmd::TriggerCapture);
        assert_eq!(decoded_capture.preset_key, "generic_60_2");
        assert_eq!(decoded_capture.sync_state, "locked");
        assert_eq!(
            decoded_capture.primary_samples.len(),
            decoded_capture.secondary_samples.len()
        );
        assert!(decoded_capture.primary_edge_count > 0);

        let directory = runtime.handle_packet(Packet::new(Cmd::GetTriggerDecoderDirectory, vec![]));
        let presets = decode_trigger_decoder_directory_payload(&directory.payload).unwrap();
        assert_eq!(directory.cmd, Cmd::TriggerDecoderDirectory);
        assert!(presets.iter().any(|preset| preset.key == "honda_k20_12_1"));
        assert!(presets.iter().any(|preset| preset.key == "subaru_6_7"));
        assert!(presets.iter().any(|preset| preset.key == "mitsubishi_4g63"));
        assert!(presets.iter().any(|preset| preset.key == "dual_wheel"));
        assert!(presets
            .iter()
            .any(|preset| preset.pattern_kind == "missing_tooth"));

        let tooth_log = runtime.handle_packet(Packet::new(Cmd::GetTriggerToothLog, vec![]));
        let decoded_tooth_log = decode_trigger_tooth_log_payload(&tooth_log.payload).unwrap();
        assert_eq!(tooth_log.cmd, Cmd::TriggerToothLog);
        assert_eq!(decoded_tooth_log.preset_key, "generic_60_2");
        assert!(!decoded_tooth_log.tooth_intervals_us.is_empty());
        assert!(
            decoded_tooth_log.reference_event_index
                < decoded_tooth_log.tooth_intervals_us.len() as u16
        );
        assert!(decoded_tooth_log.dominant_gap_ratio >= 1.0);
    }

    #[test]
    fn writing_page0_reconfigures_runtime_trigger_decoder() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let page_id = ConfigPage::BaseEngineFuelComm as u8;
        let page_len = runtime.store.page_length(page_id).unwrap();
        let mut page = vec![0u8; page_len];
        page[0..4].copy_from_slice(b"STC2");
        page[4] = 1;
        page[5] = 8; // engine cylinders

        // Distributor type should map by cylinder count (4/6/8 dynamic mapping).
        page[44] = 11;
        let distributor_write = runtime.handle_packet(Packet::new(
            Cmd::WritePage,
            encode_page_payload(page_id, &page),
        ));
        assert_eq!(distributor_write.cmd, Cmd::Ack);
        let distributor_capture =
            runtime.handle_packet(Packet::new(Cmd::GetTriggerCapture, vec![]));
        let decoded_distributor =
            decode_trigger_capture_payload(&distributor_capture.payload).unwrap();
        assert_eq!(decoded_distributor.preset_key, "distributor_basic_8");

        // Single-tooth + secondary should map into dual-wheel runtime mode.
        page[44] = 13;
        page[46..48].copy_from_slice(&24u16.to_be_bytes());
        page[48..50].copy_from_slice(&0u16.to_be_bytes());
        page[57] = 1; // secondary enabled
        let dual_write = runtime.handle_packet(Packet::new(
            Cmd::WritePage,
            encode_page_payload(page_id, &page),
        ));
        assert_eq!(dual_write.cmd, Cmd::Ack);
        let dual_capture = runtime.handle_packet(Packet::new(Cmd::GetTriggerCapture, vec![]));
        let decoded_dual = decode_trigger_capture_payload(&dual_capture.payload).unwrap();
        assert_eq!(decoded_dual.preset_key, "dual_wheel");
        assert!(decoded_dual.secondary_edge_count > 0);

        // Missing-tooth 36-1 should map to generic_36_1.
        page[44] = 0;
        page[46..48].copy_from_slice(&36u16.to_be_bytes());
        page[48..50].copy_from_slice(&1u16.to_be_bytes());
        page[57] = 0;
        let mt_write = runtime.handle_packet(Packet::new(
            Cmd::WritePage,
            encode_page_payload(page_id, &page),
        ));
        assert_eq!(mt_write.cmd, Cmd::Ack);
        let mt_capture = runtime.handle_packet(Packet::new(Cmd::GetTriggerCapture, vec![]));
        let decoded_mt = decode_trigger_capture_payload(&mt_capture.payload).unwrap();
        assert_eq!(decoded_mt.preset_key, "generic_36_1");
        assert!(decoded_mt.sync_gap_tooth_count >= 1);
    }

    #[test]
    fn runtime_datalog_lifecycle_exposes_status_and_blocks() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let start = runtime.handle_packet(Packet::new(Cmd::LogStart, vec![]));
        assert_eq!(start.cmd, Cmd::Ack);

        for _ in 0..12 {
            let _ = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
        }

        let status = runtime.handle_packet(Packet::new(Cmd::LogStatus, vec![]));
        let decoded_status = decode_log_status_payload(&status.payload).unwrap();
        assert_eq!(status.cmd, Cmd::LogStatusResponse);
        assert!(decoded_status.active);
        assert!(decoded_status.storage_present);
        assert!(decoded_status.block_count >= 1);
        assert_eq!(decoded_status.block_size, 256);

        let read_block = runtime.handle_packet(Packet::new(Cmd::ReadLogBlock, vec![0x00, 0x00]));
        let decoded_block = decode_log_block_payload(&read_block.payload).unwrap();
        assert_eq!(read_block.cmd, Cmd::LogBlockData);
        assert_eq!(decoded_block.block_index, 0);
        assert_eq!(decoded_block.total_blocks, decoded_status.block_count);
        assert!(!decoded_block.payload.is_empty());

        let stop = runtime.handle_packet(Packet::new(Cmd::LogStop, vec![]));
        assert_eq!(stop.cmd, Cmd::Ack);

        let status_after = runtime.handle_packet(Packet::new(Cmd::LogStatus, vec![]));
        let decoded_after = decode_log_status_payload(&status_after.payload).unwrap();
        assert!(!decoded_after.active);
        assert!(decoded_after.logbook_entries >= 1);
    }

    #[test]
    fn runtime_logbook_summary_reset_and_rtc_sync() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();

        let initial_summary = runtime.handle_packet(Packet::new(Cmd::GetLogbookSummary, vec![]));
        let decoded_initial = decode_logbook_summary_payload(&initial_summary.payload).unwrap();
        assert_eq!(initial_summary.cmd, Cmd::LogbookSummaryResponse);
        assert_eq!(decoded_initial.sessions, 0);
        assert_eq!(decoded_initial.entries, 0);
        assert!(!decoded_initial.rtc_synced);

        assert_eq!(
            runtime
                .handle_packet(Packet::new(Cmd::LogStart, vec![]))
                .cmd,
            Cmd::Ack
        );
        for _ in 0..10 {
            let _ = runtime.handle_packet(Packet::new(Cmd::GetLiveData, vec![]));
        }
        assert_eq!(
            runtime.handle_packet(Packet::new(Cmd::LogStop, vec![])).cmd,
            Cmd::Ack
        );

        let summary_after_log = runtime.handle_packet(Packet::new(Cmd::GetLogbookSummary, vec![]));
        let decoded_after_log = decode_logbook_summary_payload(&summary_after_log.payload).unwrap();
        assert_eq!(decoded_after_log.sessions, 1);
        assert_eq!(decoded_after_log.entries, 1);
        assert_eq!(decoded_after_log.last_session_id, 1);
        assert!(decoded_after_log.last_elapsed_ms > 0);
        assert!(decoded_after_log.last_bytes_written > 0);
        assert!(decoded_after_log.last_block_count > 0);

        let sync = runtime.handle_packet(Packet::new(
            Cmd::SyncRtc,
            encode_sync_rtc_payload(1_710_000_123_456),
        ));
        assert_eq!(sync.cmd, Cmd::Ack);
        let synced_summary = runtime.handle_packet(Packet::new(Cmd::GetLogbookSummary, vec![]));
        let decoded_synced = decode_logbook_summary_payload(&synced_summary.payload).unwrap();
        assert!(decoded_synced.rtc_synced);
        assert_eq!(decoded_synced.last_rtc_sync_ms, u32::MAX);

        let reset = runtime.handle_packet(Packet::new(Cmd::ResetLogbook, vec![]));
        assert_eq!(reset.cmd, Cmd::Ack);

        let summary_after_reset =
            runtime.handle_packet(Packet::new(Cmd::GetLogbookSummary, vec![]));
        let decoded_after_reset =
            decode_logbook_summary_payload(&summary_after_reset.payload).unwrap();
        assert_eq!(decoded_after_reset.sessions, 0);
        assert_eq!(decoded_after_reset.entries, 0);
        assert_eq!(decoded_after_reset.total_elapsed_ms, 0);
        assert_eq!(decoded_after_reset.total_bytes_written, 0);
        assert_eq!(decoded_after_reset.last_session_id, 0);
        assert_eq!(decoded_after_reset.last_block_count, 0);
        assert!(decoded_after_reset.rtc_synced);
    }

    #[test]
    fn runtime_rejects_invalid_rtc_sync_and_reset_while_logging() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        assert_eq!(
            runtime
                .handle_packet(Packet::new(Cmd::LogStart, vec![]))
                .cmd,
            Cmd::Ack
        );

        let reset_while_logging = runtime.handle_packet(Packet::new(Cmd::ResetLogbook, vec![]));
        assert_eq!(reset_while_logging.cmd, Cmd::Nack);
        let (_, reason) = decode_nack_payload(&reset_while_logging.payload).unwrap();
        assert!(reason.contains("cannot reset"));

        let bad_sync = runtime.handle_packet(Packet::new(Cmd::SyncRtc, vec![0x00, 0x01]));
        assert_eq!(bad_sync.cmd, Cmd::Nack);
        let (_, bad_reason) = decode_nack_payload(&bad_sync.payload).unwrap();
        assert!(bad_reason.contains("sync-rtc"));
    }

    #[test]
    fn runtime_rejects_invalid_log_block_request() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let response = runtime.handle_packet(Packet::new(Cmd::ReadLogBlock, vec![0x00]));
        assert_eq!(response.cmd, Cmd::Nack);
        let (code, reason) = decode_nack_payload(&response.payload).unwrap();
        assert_eq!(code, 2);
        assert!(reason.contains("read-log-block"));
    }

    #[test]
    fn runtime_seeds_pin_assignment_page_from_defaults() {
        let runtime = FirmwareRuntime::new_ecu_v1();
        let payload = runtime
            .store
            .read_page(ConfigPage::PinAssignment as u8)
            .unwrap();
        let decoded = deserialize_assignments_from_page(payload).unwrap();

        assert_eq!(decoded.len(), runtime.pin_assignments.len());
        assert_eq!(
            runtime.store.needs_burn(ConfigPage::PinAssignment as u8),
            Some(false)
        );
    }

    #[test]
    fn writing_pin_assignment_page_updates_runtime_assignments() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let page_len = runtime
            .store
            .page_length(ConfigPage::PinAssignment as u8)
            .unwrap();
        let new_assignments = apply_assignment_overrides(
            &runtime.pin_assignments,
            &[PinAssignmentRequest {
                function: EcuFunction::BoostControl,
                pin_id: "PC8",
            }],
        )
        .unwrap();
        let page = serialize_assignments_to_page(&new_assignments, page_len).unwrap();

        let response = runtime.handle_packet(Packet::new(
            Cmd::WritePage,
            encode_page_payload(ConfigPage::PinAssignment as u8, &page),
        ));

        assert_eq!(response.cmd, Cmd::Ack);
        assert_eq!(
            runtime
                .pin_assignment(EcuFunction::BoostControl)
                .unwrap()
                .pin_id,
            "PC8"
        );
        assert_eq!(
            runtime.store.needs_burn(ConfigPage::PinAssignment as u8),
            Some(true)
        );
    }

    #[test]
    fn invalid_pin_assignment_page_is_rejected_and_reverted() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let page_len = runtime
            .store
            .page_length(ConfigPage::PinAssignment as u8)
            .unwrap();
        let mut invalid_page = vec![0u8; page_len];
        invalid_page[0..4].copy_from_slice(b"STIO");
        invalid_page[4] = 1;
        invalid_page[5] = 1;
        invalid_page[6] = EcuFunction::BoostControl.code();
        invalid_page[7] = 3;
        invalid_page[8..11].copy_from_slice(b"PA0");

        let response = runtime.handle_packet(Packet::new(
            Cmd::WritePage,
            encode_page_payload(ConfigPage::PinAssignment as u8, &invalid_page),
        ));

        assert_eq!(response.cmd, Cmd::Nack);
        assert_eq!(
            runtime
                .pin_assignment(EcuFunction::BoostControl)
                .unwrap()
                .pin_id,
            "PB0"
        );
    }

    #[test]
    fn runtime_reports_page_statuses_for_software_sync() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        runtime
            .apply_pin_assignment_overrides(&[PinAssignmentRequest {
                function: EcuFunction::BoostControl,
                pin_id: "PC8",
            }])
            .unwrap();

        let response = runtime.handle_packet(Packet::new(Cmd::GetPageStatuses, vec![]));
        let statuses = decode_page_statuses_payload(&response.payload).unwrap();

        assert_eq!(response.cmd, Cmd::PageStatuses);
        assert_eq!(statuses.len(), 10);
        assert!(statuses
            .iter()
            .any(|status| status.page_id == ConfigPage::PinAssignment as u8 && status.needs_burn));
    }

    #[test]
    fn runtime_reports_network_profile() {
        let mut runtime = FirmwareRuntime::new_ecu_v1();
        let response = runtime.handle_packet(Packet::new(Cmd::GetNetworkProfile, vec![]));
        let profile = decode_network_profile_payload(&response.payload).unwrap();

        assert_eq!(response.cmd, Cmd::NetworkProfile);
        assert_eq!(profile.product_track, ProductTrack::HeadlessEcu);
        assert!(profile.links.iter().any(|link| {
            link.kind == TransportLinkKind::UsbSerial
                && link.classes.contains(&MessageClass::FirmwareUpdate)
        }));
    }
}
