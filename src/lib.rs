#![forbid(unsafe_code)]

// ─── Existing modules ─────────────────────────────────────────────────────────
pub mod board;
pub mod boot;
pub mod config;
pub mod contract;
pub mod diagnostics;
pub mod engine;
pub mod io;
pub mod live_data;
pub mod mcu;
pub mod network;
pub mod pinmux;
pub mod protection;
pub mod protocol;
pub mod reset_reason;
pub mod transport;
pub mod trigger;
pub mod rotational_idle;
pub mod tcu;
pub mod wideband;

// ─── New engine algorithm modules ─────────────────────────────────────────────
pub mod fuel;
pub mod ignition;
pub mod knock;
pub mod knock_ml;
pub mod lambda_dfsdm;
pub mod idle;
pub mod boost;
pub mod launch;
pub mod antilag;
pub mod traction;
pub mod dbw;
pub mod vvt;
pub mod crypto;

// ─── HAL abstraction ─────────────────────────────────────────────────────────
pub mod hal;

pub use board::{
    assignable_pins, board_definition, board_matches_firmware_identity, find_pin,
    validate_pin_assignment, BoardDefinition, BoardPathKind, BoardValidationError, ElectricalClass,
    PinCapability, ST_ECU_V1_BOARD, ST_ECU_V1_PINS,
};
pub use config::{
    ConfigImageError, ConfigPage, ConfigPageHeader, ConfigPageStatus, ConfigStore,
    ConfigStoreError, CONFIG_IMAGE_MAGIC, PAGE_DIRECTORY,
};
pub use contract::{
    base_capabilities, Capability, CapabilityParseError, FirmwareCompatibility, FirmwareIdentity,
    PageDirectoryEntry, TableDirectoryEntry, CONFIG_FORMAT_VERSION, PROTOCOL_VERSION,
    SCHEMA_VERSION, TABLE_DIRECTORY,
};
pub use diagnostics::{DtcCode, DtcSeverity, FreezeFrame, FreezeFrameHeader, SAMPLE_FREEZE_FRAMES};
pub use io::{
    apply_assignment_overrides, default_pin_assignments, deserialize_assignments_from_page,
    serialize_assignments_to_page, validate_assignment_set, AssignmentError, EcuFunction,
    EcuFunctionParseError, PinAssignmentRequest, ResolvedPinAssignment, RoutingPolicy,
};
pub use live_data::{LiveDataFrame, LIVE_DATA_SIZE, status, protect, error};
pub use mcu::{
    find_mcu_pin, mcu_definition, McuDefinition, McuPackage, McuPinCapability,
    STM32H743ZG_SELECTED_MATRIX, STM32H743ZG_SELECTED_PINS,
};
pub use network::{
    display_network_profile, headless_network_profile, preferred_links, supports_message,
    LinkContract, MessageClass, MessageClassParseError, NetworkNodeKind, NetworkProfile,
    ProductTrack, ProductTrackParseError, TransportLinkKind, TransportLinkParseError,
    DISPLAY_PROFILE, HEADLESS_PROFILE,
};
pub use pinmux::{PinFunctionClass, PinFunctionClassParseError, PinRoute};
pub use protocol::{
    decode_ack_payload, decode_capabilities_payload, decode_freeze_frames_payload,
    decode_identity_payload, decode_nack_payload, decode_network_profile_payload,
    decode_page_payload, decode_page_request, decode_page_statuses_payload,
    decode_pin_assignments_payload, decode_pin_directory_payload, decode_raw_table_payload,
    decode_sensor_raw_directory_payload, decode_sensor_raw_payload, decode_table_metadata_payload,
    decode_trigger_capture_payload, decode_trigger_decoder_directory_payload,
    decode_trigger_tooth_log_payload, encode_ack_payload, encode_capabilities_payload,
    encode_freeze_frames_payload, encode_identity_payload, encode_nack_payload,
    encode_network_profile_payload, encode_page_directory_payload, encode_page_payload,
    encode_page_request, encode_page_statuses_payload, encode_pin_assignments_payload,
    encode_pin_directory_payload, encode_sensor_raw_directory_payload, encode_sensor_raw_payload,
    encode_table_directory_payload, encode_table_metadata_payload, encode_trigger_capture_payload,
    encode_trigger_decoder_directory_payload, encode_trigger_tooth_log_payload, Cmd,
    DecodedFreezeFrame, DecodedIdentity, DecodedNetworkLink, DecodedNetworkProfile,
    DecodedPagePayload, DecodedPageStatus, DecodedPinAssignment, DecodedPinDirectoryEntry,
    DecodedPinRoute, DecodedSensorRaw, DecodedSensorRawDirectoryEntry, DecodedTableMetadataEntry,
    DecodedTriggerCapture, DecodedTriggerDecoderPreset, DecodedTriggerToothLog, Packet,
    ProtocolError, RawTablePayload, SensorRawDirectoryEntry,
};
pub use transport::{FirmwareRuntime, RuntimeNackCode, TransportCapabilities, TransportKind};
pub use trigger::{
    sample_trigger_capture, sample_trigger_tooth_log, TriggerCapture, TriggerDecoderPreset,
    TriggerToothLog, SUPPORTED_TRIGGER_DECODERS,
};
pub use rotational_idle::{RotationalIdleRuntime, RotationalIdleSample};
pub use tcu::{ExternalTcuRuntime, TransmissionSample};
pub use wideband::{WidebandRuntime, WidebandSample, WidebandSource};

// ─── New module re-exports ────────────────────────────────────────────────────
pub use engine::{
    EngineRuntime, SyncState, FuelState as EngineFuelState, IgnitionState as EngineIgnState,
    KnockState, IdleState as EngineIdleState, BoostState as EngineBoostState,
    VvtState as EngineVvtState, DbwState as EngineDbwState, DbwProfile as EngineDbwProfile,
    LaunchState, LaunchPreset, LaunchMode, LaunchPhase,
    AntiLagState, AlsMode as EngineAlsMode,
    TractionState as EngineTractionState, TractionPreset as EngineTcPreset,
    ProtectionState as EngineProtState, PerCylinderTrim,
};
pub use fuel::{FuelCalculator, FuelConfig, VeTable, LtftMap, WallWettingState};
pub use ignition::{IgnitionScheduler, IgnitionConfig, IgnitionTable, DwellTable, IgnitionMode};
pub use knock::{SoftKnockDetector, FftKnockDetector, KnockConfig, KnockLearningMap};
pub use idle::{IdleController, IdleConfig};
pub use boost::{BoostController, BoostConfig, BoostByGearTable};
pub use launch::{LaunchController, MAX_LAUNCH_PRESETS};
pub use antilag::{AntiLagController, AlsConfig, AlsMode};
pub use traction::{TractionController, TcPreset, MAX_TC_PRESETS};
pub use dbw::{DbwController, DbwConfig, PedalMap, DbwProfile};
pub use vvt::{VvtController, VvtConfig, VvtTargetTable, HydraulicFeedForward};
pub use protection::{ProtectionManager, ProtectionConfig, SoftwareWatchdog, ProtectionAction};
pub use crypto::{TuneEncryption, TuneKey, CryptoError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_ids_are_unique() {
        let mut ids: Vec<u8> = PAGE_DIRECTORY.iter().map(|page| page.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PAGE_DIRECTORY.len());
    }

    #[test]
    fn live_data_size_matches_contract() {
        assert_eq!(LIVE_DATA_SIZE, 128);
    }

    #[test]
    fn identity_defaults_use_current_contract_versions() {
        let identity = FirmwareIdentity::simulator();
        assert_eq!(identity.protocol_version, PROTOCOL_VERSION);
        assert_eq!(identity.schema_version, SCHEMA_VERSION);
        assert!(identity.signature.contains("ST"));
    }

    #[test]
    fn base_capabilities_are_not_empty() {
        assert!(!base_capabilities(false).is_empty());
    }

    // ─── Integration smoke tests ──────────────────────────────────────────────

    #[test]
    fn live_data_frame_encodes_128_bytes() {
        let frame = LiveDataFrame::default();
        let enc = frame.encode();
        assert_eq!(enc.len(), LIVE_DATA_SIZE);
    }

    #[test]
    fn engine_runtime_initializes_clean() {
        let rt = EngineRuntime::default();
        assert_eq!(rt.sync_state, SyncState::Unsynced);
        assert_eq!(rt.rpm, 0.0);
    }

    #[test]
    fn fuel_calculator_creates() {
        let _calc = FuelCalculator::new(FuelConfig::default());
    }

    #[test]
    fn ignition_scheduler_creates() {
        let _sched = IgnitionScheduler::new(IgnitionConfig::default());
    }

    #[test]
    fn launch_controller_has_five_presets() {
        let ctrl = LaunchController::default();
        assert_eq!(ctrl.presets.len(), MAX_LAUNCH_PRESETS);
    }

    #[test]
    fn traction_controller_has_three_presets() {
        let ctrl = TractionController::default();
        assert_eq!(ctrl.presets.len(), MAX_TC_PRESETS);
    }

    #[test]
    fn protection_no_fault_on_normal_conditions() {
        use crate::protection::ProtectionState;
        let mgr = ProtectionManager::new(ProtectionConfig::default());
        let mut state = ProtectionState::default();
        let action = mgr.evaluate(&mut state, 3000.0, 120.0, 300.0, 85.0, 14.7, 800.0, 800.0);
        assert_eq!(action, ProtectionAction::None);
    }

    #[test]
    fn crypto_f407_not_supported() {
        let enc = TuneEncryption::new_f407();
        assert!(!enc.target_supports_crypto);
    }

    #[test]
    fn boost_gear_table_gear1_less_than_gear3() {
        let table = BoostByGearTable::default();
        assert!(table.target_for(3, 4000.0) > table.target_for(1, 4000.0));
    }
}
