#![forbid(unsafe_code)]

pub mod boot;
pub mod config;
pub mod contract;
pub mod diagnostics;
pub mod engine;
pub mod live_data;
pub mod protection;
pub mod reset_reason;
pub mod transport;

pub use config::{ConfigPage, PAGE_DIRECTORY};
pub use contract::{
    Capability,
    FirmwareCompatibility,
    FirmwareIdentity,
    PageDirectoryEntry,
    TableDirectoryEntry,
    PROTOCOL_VERSION,
    SCHEMA_VERSION,
};
pub use live_data::{LiveDataFrame, LIVE_DATA_SIZE};

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
}
