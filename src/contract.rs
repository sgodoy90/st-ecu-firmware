#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    LiveData,
    PageRead,
    PageWrite,
    PageBurn,
    PageStatus,
    TableRead,
    TableWrite,
    TableCellWrite,
    TableMetadata,
    Dtc,
    SensorRaw,
    SensorRawDirectory,
    FreezeFrame,
    TriggerCapture,
    TriggerDecoderDirectory,
    TriggerToothLog,
    OutputTest,
    OutputTestDirectory,
    FirmwareFlash,
    Page0Config,
    PinDirectory,
    PinAssignment,
    NetworkProfile,
    UsbSerial,
    CanFd,
    WifiBridge,
    ExternalTcu,
    TorqueIntervention,
    WidebandController,
    DatalogStorage,
    Logbook,
    RtcClock,
    LogBlockRead,
    DisplayLink,
    DashboardFrames,
    Simulator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityParseError {
    pub code: u8,
}

impl Capability {
    pub const fn code(self) -> u8 {
        match self {
            Self::LiveData => 0x01,
            Self::PageRead => 0x02,
            Self::PageWrite => 0x03,
            Self::PageBurn => 0x04,
            Self::PageStatus => 0x05,
            Self::TableRead => 0x06,
            Self::TableWrite => 0x07,
            Self::TableCellWrite => 0x08,
            Self::TableMetadata => 0x09,
            Self::Dtc => 0x0A,
            Self::SensorRaw => 0x0B,
            Self::SensorRawDirectory => 0x19,
            Self::FreezeFrame => 0x1A,
            Self::TriggerCapture => 0x1B,
            Self::TriggerDecoderDirectory => 0x1C,
            Self::TriggerToothLog => 0x1D,
            Self::OutputTest => 0x0C,
            Self::OutputTestDirectory => 0x18,
            Self::FirmwareFlash => 0x0D,
            Self::Page0Config => 0x0E,
            Self::PinDirectory => 0x0F,
            Self::PinAssignment => 0x10,
            Self::NetworkProfile => 0x11,
            Self::UsbSerial => 0x12,
            Self::CanFd => 0x13,
            Self::WifiBridge => 0x14,
            Self::ExternalTcu => 0x1E,
            Self::TorqueIntervention => 0x1F,
            Self::WidebandController => 0x20,
            Self::DatalogStorage => 0x21,
            Self::Logbook => 0x22,
            Self::RtcClock => 0x23,
            Self::LogBlockRead => 0x24,
            Self::DisplayLink => 0x15,
            Self::DashboardFrames => 0x16,
            Self::Simulator => 0x17,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::LiveData => "live_data",
            Self::PageRead => "page_read",
            Self::PageWrite => "page_write",
            Self::PageBurn => "page_burn",
            Self::PageStatus => "page_status",
            Self::TableRead => "table_read",
            Self::TableWrite => "table_write",
            Self::TableCellWrite => "table_cell_write",
            Self::TableMetadata => "table_metadata",
            Self::Dtc => "dtc",
            Self::SensorRaw => "sensor_raw",
            Self::SensorRawDirectory => "sensor_raw_directory",
            Self::FreezeFrame => "freeze_frame",
            Self::TriggerCapture => "trigger_capture",
            Self::TriggerDecoderDirectory => "trigger_decoder_directory",
            Self::TriggerToothLog => "trigger_tooth_log",
            Self::OutputTest => "output_test",
            Self::OutputTestDirectory => "output_test_directory",
            Self::FirmwareFlash => "firmware_flash",
            Self::Page0Config => "page_0_config",
            Self::PinDirectory => "pin_directory",
            Self::PinAssignment => "pin_assignment",
            Self::NetworkProfile => "network_profile",
            Self::UsbSerial => "usb_serial",
            Self::CanFd => "can_fd",
            Self::WifiBridge => "wifi_bridge",
            Self::ExternalTcu => "external_tcu",
            Self::TorqueIntervention => "torque_intervention",
            Self::WidebandController => "wideband_controller",
            Self::DatalogStorage => "datalog_storage",
            Self::Logbook => "logbook",
            Self::RtcClock => "rtc_clock",
            Self::LogBlockRead => "log_block_read",
            Self::DisplayLink => "display_link",
            Self::DashboardFrames => "dashboard_frames",
            Self::Simulator => "simulator",
        }
    }
}

impl TryFrom<u8> for Capability {
    type Error = CapabilityParseError;

    fn try_from(value: u8) -> Result<Self, CapabilityParseError> {
        match value {
            0x01 => Ok(Self::LiveData),
            0x02 => Ok(Self::PageRead),
            0x03 => Ok(Self::PageWrite),
            0x04 => Ok(Self::PageBurn),
            0x05 => Ok(Self::PageStatus),
            0x06 => Ok(Self::TableRead),
            0x07 => Ok(Self::TableWrite),
            0x08 => Ok(Self::TableCellWrite),
            0x09 => Ok(Self::TableMetadata),
            0x0A => Ok(Self::Dtc),
            0x0B => Ok(Self::SensorRaw),
            0x19 => Ok(Self::SensorRawDirectory),
            0x1A => Ok(Self::FreezeFrame),
            0x1B => Ok(Self::TriggerCapture),
            0x1C => Ok(Self::TriggerDecoderDirectory),
            0x1D => Ok(Self::TriggerToothLog),
            0x0C => Ok(Self::OutputTest),
            0x18 => Ok(Self::OutputTestDirectory),
            0x0D => Ok(Self::FirmwareFlash),
            0x0E => Ok(Self::Page0Config),
            0x0F => Ok(Self::PinDirectory),
            0x10 => Ok(Self::PinAssignment),
            0x11 => Ok(Self::NetworkProfile),
            0x12 => Ok(Self::UsbSerial),
            0x13 => Ok(Self::CanFd),
            0x14 => Ok(Self::WifiBridge),
            0x1E => Ok(Self::ExternalTcu),
            0x1F => Ok(Self::TorqueIntervention),
            0x20 => Ok(Self::WidebandController),
            0x21 => Ok(Self::DatalogStorage),
            0x22 => Ok(Self::Logbook),
            0x23 => Ok(Self::RtcClock),
            0x24 => Ok(Self::LogBlockRead),
            0x15 => Ok(Self::DisplayLink),
            0x16 => Ok(Self::DashboardFrames),
            0x17 => Ok(Self::Simulator),
            _ => Err(CapabilityParseError { code: value }),
        }
    }
}

pub const PROTOCOL_VERSION: u8 = 1;
pub const SCHEMA_VERSION: u16 = 1;
pub const CONFIG_FORMAT_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareIdentity {
    pub firmware_id: &'static str,
    pub firmware_semver: &'static str,
    pub schema_version: u16,
    pub protocol_version: u8,
    pub board_id: &'static str,
    pub serial: &'static str,
    pub signature: &'static str,
}

impl FirmwareIdentity {
    pub const fn simulator() -> Self {
        Self {
            firmware_id: "st-simulator",
            firmware_semver: "0.1.0",
            schema_version: SCHEMA_VERSION,
            protocol_version: PROTOCOL_VERSION,
            board_id: "st-sim-v1",
            serial: "SIM00000001",
            signature: "ST-SIM-v1",
        }
    }

    pub const fn ecu_v1() -> Self {
        Self {
            firmware_id: "st-ecu-runtime",
            firmware_semver: "0.1.0",
            schema_version: SCHEMA_VERSION,
            protocol_version: PROTOCOL_VERSION,
            board_id: "st-ecu-v1",
            serial: "ST00000001",
            signature: "ST-ECU-v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareCompatibility {
    pub compatible: bool,
    pub protocol_version: u8,
    pub schema_version: u16,
}

impl FirmwareCompatibility {
    pub const fn current() -> Self {
        Self {
            compatible: true,
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
        }
    }

    pub const fn from_identity(identity: &FirmwareIdentity) -> Self {
        Self {
            compatible: identity.protocol_version == PROTOCOL_VERSION
                && identity.schema_version == SCHEMA_VERSION,
            protocol_version: identity.protocol_version,
            schema_version: identity.schema_version,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageDirectoryEntry {
    pub id: u8,
    pub key: &'static str,
    pub byte_length: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableDirectoryEntry {
    pub id: u8,
    pub key: &'static str,
    pub x_count: u8,
    pub y_count: u8,
    pub signed: bool,
    pub x_scale: u16,
    pub y_scale: u16,
    pub value_scale: u16,
    pub x_label: &'static str,
    pub y_label: &'static str,
    pub z_label: &'static str,
    pub unit: &'static str,
    pub min_value_raw: i32,
    pub max_value_raw: i32,
    pub default_value_raw: i32,
}

pub const TABLE_DIRECTORY: [TableDirectoryEntry; 8] = [
    TableDirectoryEntry {
        id: 0x00,
        key: "ve",
        x_count: 48,
        y_count: 48,
        signed: false,
        x_scale: 1,
        y_scale: 10,
        value_scale: 100,
        x_label: "RPM",
        y_label: "MAP",
        z_label: "VE",
        unit: "%",
        min_value_raw: 0,
        max_value_raw: 15000,
        default_value_raw: 5000,
    },
    TableDirectoryEntry {
        id: 0x01,
        key: "ignition",
        x_count: 48,
        y_count: 48,
        signed: true,
        x_scale: 1,
        y_scale: 10,
        value_scale: 10,
        x_label: "RPM",
        y_label: "MAP",
        z_label: "Ignition Timing",
        unit: "deg",
        min_value_raw: -200,
        max_value_raw: 900,
        default_value_raw: 150,
    },
    TableDirectoryEntry {
        id: 0x02,
        key: "afr_target",
        x_count: 48,
        y_count: 48,
        signed: false,
        x_scale: 1,
        y_scale: 10,
        value_scale: 10000,
        x_label: "RPM",
        y_label: "MAP",
        z_label: "Lambda Target",
        unit: "lambda",
        min_value_raw: 6500,
        max_value_raw: 15000,
        default_value_raw: 10000,
    },
    TableDirectoryEntry {
        id: 0x03,
        key: "boost_target",
        x_count: 48,
        y_count: 48,
        signed: true,
        x_scale: 1,
        y_scale: 100,
        value_scale: 10,
        x_label: "RPM",
        y_label: "TPS",
        z_label: "Boost Target",
        unit: "kPa",
        min_value_raw: -1000,
        max_value_raw: 4000,
        default_value_raw: 0,
    },
    TableDirectoryEntry {
        id: 0x04,
        key: "vvt_b1_intake",
        x_count: 8,   // VvtTargetTable in vvt.rs is 8×8
        y_count: 8,
        signed: true,
        x_scale: 1,
        y_scale: 10,
        value_scale: 10,
        x_label: "RPM",
        y_label: "MAP",
        z_label: "VVT Intake Target",
        unit: "deg",
        min_value_raw: -700,
        max_value_raw: 700,
        default_value_raw: 0,
    },
    TableDirectoryEntry {
        id: 0x05,
        key: "vvt_b1_exhaust",
        x_count: 8,   // VvtTargetTable in vvt.rs is 8×8
        y_count: 8,
        signed: true,
        x_scale: 1,
        y_scale: 10,
        value_scale: 10,
        x_label: "RPM",
        y_label: "MAP",
        z_label: "VVT Exhaust Target",
        unit: "deg",
        min_value_raw: -700,
        max_value_raw: 700,
        default_value_raw: 0,
    },
    TableDirectoryEntry {
        id: 0x10,
        key: "boost_duty",
        x_count: 16,
        y_count: 16,
        signed: false,
        x_scale: 1,
        y_scale: 10,
        value_scale: 10,
        x_label: "RPM",
        y_label: "Boost",
        z_label: "WG Duty",
        unit: "%",
        min_value_raw: 0,
        max_value_raw: 1000,
        default_value_raw: 0,
    },
    TableDirectoryEntry {
        id: 0x12,
        key: "staging",
        x_count: 16,
        y_count: 16,
        signed: false,
        x_scale: 1,
        y_scale: 10,
        value_scale: 10,
        x_label: "RPM",
        y_label: "Load",
        z_label: "Secondary Staging",
        unit: "%",
        min_value_raw: 0,
        max_value_raw: 1000,
        default_value_raw: 0,
    },
];

const BASE_CAPABILITIES: [Capability; 33] = [
    Capability::LiveData,
    Capability::PageRead,
    Capability::PageWrite,
    Capability::PageBurn,
    Capability::PageStatus,
    Capability::TableRead,
    Capability::TableWrite,
    Capability::TableCellWrite,
    Capability::TableMetadata,
    Capability::Dtc,
    Capability::SensorRaw,
    Capability::SensorRawDirectory,
    Capability::FreezeFrame,
    Capability::TriggerCapture,
    Capability::TriggerDecoderDirectory,
    Capability::TriggerToothLog,
    Capability::OutputTest,
    Capability::OutputTestDirectory,
    Capability::FirmwareFlash,
    Capability::Page0Config,
    Capability::PinDirectory,
    Capability::PinAssignment,
    Capability::NetworkProfile,
    Capability::UsbSerial,
    Capability::CanFd,
    Capability::WifiBridge,
    Capability::ExternalTcu,
    Capability::TorqueIntervention,
    Capability::WidebandController,
    Capability::DatalogStorage,
    Capability::Logbook,
    Capability::RtcClock,
    Capability::LogBlockRead,
];

pub fn base_capabilities(simulator: bool) -> Vec<Capability> {
    let mut capabilities = BASE_CAPABILITIES.to_vec();
    if simulator {
        capabilities.push(Capability::Simulator);
    }
    capabilities
}

pub fn supports_capability(capabilities: &[Capability], wanted: Capability) -> bool {
    capabilities.contains(&wanted)
}
