#![forbid(unsafe_code)]

pub mod board;
pub mod boot;
pub mod config;
pub mod contract;
pub mod diagnostics;
pub mod engine;
pub mod io;
pub mod live_data;
pub mod protection;
pub mod protocol;
pub mod reset_reason;
pub mod transport;

pub use board::{
    assignable_pins, board_definition, board_matches_firmware_identity, find_pin,
    validate_pin_assignment, BoardDefinition, BoardValidationError, ElectricalClass, PinCapability,
    PinFunctionClass, ST_ECU_V1_BOARD, ST_ECU_V1_PINS,
};
pub use config::{ConfigPage, ConfigStore, PAGE_DIRECTORY};
pub use contract::{
    base_capabilities, Capability, CapabilityParseError, FirmwareCompatibility, FirmwareIdentity,
    PageDirectoryEntry, TableDirectoryEntry, PROTOCOL_VERSION, SCHEMA_VERSION, TABLE_DIRECTORY,
};
pub use io::{
    apply_assignment_overrides, default_pin_assignments, validate_assignment_set, AssignmentError,
    EcuFunction, EcuFunctionParseError, PinAssignmentRequest, ResolvedPinAssignment, RoutingPolicy,
};
pub use live_data::{LiveDataFrame, LIVE_DATA_SIZE};
pub use protocol::{
    decode_ack_payload, decode_capabilities_payload, decode_identity_payload, decode_nack_payload,
    decode_page_payload, decode_page_request, decode_pin_assignments_payload,
    decode_pin_directory_payload, encode_ack_payload, encode_capabilities_payload,
    encode_identity_payload, encode_nack_payload, encode_page_directory_payload,
    encode_page_payload, encode_page_request, encode_pin_assignments_payload,
    encode_pin_directory_payload, encode_table_directory_payload, Cmd, DecodedIdentity,
    DecodedPagePayload, DecodedPinAssignment, DecodedPinDirectoryEntry, Packet, ProtocolError,
};
pub use transport::{FirmwareRuntime, RuntimeNackCode, TransportCapabilities, TransportKind};

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
}
