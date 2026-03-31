use crate::contract::PageDirectoryEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConfigPage {
    BaseEngineFuelComm = 0,
    TriggerIgnition = 1,
    Sensors = 2,
    PinAssignment = 3,
    IdleBoostVvt = 4,
    LimitsKnock = 5,
    AdvancedAirTorque = 6,
    ProtectionsThermal = 7,
    VehicleIntegration = 8,
    UiDefaults = 9,
}

pub const PAGE_DIRECTORY: [PageDirectoryEntry; 10] = [
    PageDirectoryEntry { id: ConfigPage::BaseEngineFuelComm as u8, key: "base_engine_fuel_comm", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::TriggerIgnition as u8, key: "trigger_ignition", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::Sensors as u8, key: "sensors", byte_length: 1024 },
    PageDirectoryEntry { id: ConfigPage::PinAssignment as u8, key: "pin_assignment", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::IdleBoostVvt as u8, key: "idle_boost_vvt", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::LimitsKnock as u8, key: "limits_knock", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::AdvancedAirTorque as u8, key: "advanced_air_torque", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::ProtectionsThermal as u8, key: "protections_thermal", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::VehicleIntegration as u8, key: "vehicle_integration", byte_length: 512 },
    PageDirectoryEntry { id: ConfigPage::UiDefaults as u8, key: "ui_defaults", byte_length: 256 },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigPageHeader {
    pub page_id: u8,
    pub schema_version: u16,
    pub payload_crc: u32,
}
