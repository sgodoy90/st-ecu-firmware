#![forbid(unsafe_code)]

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
pub use io::{
    apply_assignment_overrides, default_pin_assignments, deserialize_assignments_from_page,
    serialize_assignments_to_page, validate_assignment_set, AssignmentError, EcuFunction,
    EcuFunctionParseError, PinAssignmentRequest, ResolvedPinAssignment, RoutingPolicy,
};
pub use live_data::{LiveDataFrame, LIVE_DATA_SIZE};
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
    decode_ack_payload, decode_capabilities_payload, decode_identity_payload, decode_nack_payload,
    decode_network_profile_payload, decode_page_payload, decode_page_request,
    decode_page_statuses_payload, decode_pin_assignments_payload, decode_pin_directory_payload,
    encode_ack_payload, encode_capabilities_payload, encode_identity_payload, encode_nack_payload,
    encode_network_profile_payload, encode_page_directory_payload, encode_page_payload,
    encode_page_request, encode_page_statuses_payload, encode_pin_assignments_payload,
    encode_pin_directory_payload, encode_table_directory_payload, Cmd, DecodedIdentity,
    DecodedNetworkLink, DecodedNetworkProfile, DecodedPagePayload, DecodedPageStatus,
    DecodedPinAssignment, DecodedPinDirectoryEntry, DecodedPinRoute, Packet, ProtocolError,
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
