#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    LiveData,
    PageRead,
    PageWrite,
    PageBurn,
    TableRead,
    TableWrite,
    TableCellWrite,
    Dtc,
    SensorRaw,
    OutputTest,
    FirmwareFlash,
    Page0Config,
    Simulator,
}

pub const PROTOCOL_VERSION: u8 = 1;
pub const SCHEMA_VERSION: u16 = 1;

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
