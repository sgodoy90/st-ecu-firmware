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
    Dtc,
    SensorRaw,
    OutputTest,
    FirmwareFlash,
    Page0Config,
    PinDirectory,
    PinAssignment,
    NetworkProfile,
    UsbSerial,
    CanFd,
    WifiBridge,
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
            Self::Dtc => 0x09,
            Self::SensorRaw => 0x0A,
            Self::OutputTest => 0x0B,
            Self::FirmwareFlash => 0x0C,
            Self::Page0Config => 0x0D,
            Self::PinDirectory => 0x0E,
            Self::PinAssignment => 0x0F,
            Self::NetworkProfile => 0x10,
            Self::UsbSerial => 0x11,
            Self::CanFd => 0x12,
            Self::WifiBridge => 0x13,
            Self::DisplayLink => 0x14,
            Self::DashboardFrames => 0x15,
            Self::Simulator => 0x16,
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
            Self::Dtc => "dtc",
            Self::SensorRaw => "sensor_raw",
            Self::OutputTest => "output_test",
            Self::FirmwareFlash => "firmware_flash",
            Self::Page0Config => "page_0_config",
            Self::PinDirectory => "pin_directory",
            Self::PinAssignment => "pin_assignment",
            Self::NetworkProfile => "network_profile",
            Self::UsbSerial => "usb_serial",
            Self::CanFd => "can_fd",
            Self::WifiBridge => "wifi_bridge",
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
            0x09 => Ok(Self::Dtc),
            0x0A => Ok(Self::SensorRaw),
            0x0B => Ok(Self::OutputTest),
            0x0C => Ok(Self::FirmwareFlash),
            0x0D => Ok(Self::Page0Config),
            0x0E => Ok(Self::PinDirectory),
            0x0F => Ok(Self::PinAssignment),
            0x10 => Ok(Self::NetworkProfile),
            0x11 => Ok(Self::UsbSerial),
            0x12 => Ok(Self::CanFd),
            0x13 => Ok(Self::WifiBridge),
            0x14 => Ok(Self::DisplayLink),
            0x15 => Ok(Self::DashboardFrames),
            0x16 => Ok(Self::Simulator),
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
}

pub const TABLE_DIRECTORY: [TableDirectoryEntry; 8] = [
    TableDirectoryEntry {
        id: 0x00,
        key: "ve",
        x_count: 48,
        y_count: 48,
        signed: false,
    },
    TableDirectoryEntry {
        id: 0x01,
        key: "ignition",
        x_count: 48,
        y_count: 48,
        signed: true,
    },
    TableDirectoryEntry {
        id: 0x02,
        key: "afr_target",
        x_count: 48,
        y_count: 48,
        signed: false,
    },
    TableDirectoryEntry {
        id: 0x03,
        key: "boost_target",
        x_count: 48,
        y_count: 48,
        signed: true,
    },
    TableDirectoryEntry {
        id: 0x04,
        key: "vvt_b1_intake",
        x_count: 32,
        y_count: 32,
        signed: true,
    },
    TableDirectoryEntry {
        id: 0x05,
        key: "vvt_b1_exhaust",
        x_count: 32,
        y_count: 32,
        signed: true,
    },
    TableDirectoryEntry {
        id: 0x10,
        key: "boost_duty",
        x_count: 16,
        y_count: 16,
        signed: false,
    },
    TableDirectoryEntry {
        id: 0x12,
        key: "staging",
        x_count: 16,
        y_count: 16,
        signed: false,
    },
];

const BASE_CAPABILITIES: [Capability; 19] = [
    Capability::LiveData,
    Capability::PageRead,
    Capability::PageWrite,
    Capability::PageBurn,
    Capability::PageStatus,
    Capability::TableRead,
    Capability::TableWrite,
    Capability::TableCellWrite,
    Capability::Dtc,
    Capability::SensorRaw,
    Capability::OutputTest,
    Capability::FirmwareFlash,
    Capability::Page0Config,
    Capability::PinDirectory,
    Capability::PinAssignment,
    Capability::NetworkProfile,
    Capability::UsbSerial,
    Capability::CanFd,
    Capability::WifiBridge,
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
