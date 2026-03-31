use crate::config::ConfigStore;
use crate::contract::{base_capabilities, Capability, FirmwareIdentity};
use crate::io::{
    apply_assignment_overrides, default_pin_assignments, validate_assignment_set, AssignmentError,
    PinAssignmentRequest, ResolvedPinAssignment,
};
use crate::live_data::LiveDataFrame;
use crate::protocol::{
    decode_page_payload, decode_page_request, encode_ack_payload, encode_capabilities_payload,
    encode_identity_payload, encode_nack_payload, encode_page_directory_payload,
    encode_page_payload, encode_pin_assignments_payload, encode_pin_directory_payload,
    encode_table_directory_payload, Cmd, Packet,
};

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

#[derive(Debug, Clone)]
pub struct FirmwareRuntime {
    pub identity: FirmwareIdentity,
    pub transport: TransportCapabilities,
    pub store: ConfigStore,
    pub capabilities: Vec<Capability>,
    pub pin_assignments: Vec<ResolvedPinAssignment>,
}

impl FirmwareRuntime {
    pub fn new(identity: FirmwareIdentity, simulator: bool) -> Self {
        let pin_assignments = validate_assignment_set(&default_pin_assignments())
            .expect("default pin assignments must be valid for board definition");
        Self {
            identity,
            transport: TransportCapabilities::default(),
            store: ConfigStore::new_zeroed(),
            capabilities: base_capabilities(simulator),
            pin_assignments,
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
            Cmd::GetLiveData => {
                Packet::new(Cmd::LiveData, LiveDataFrame::current().encode_payload())
            }
            Cmd::GetPageDirectory => {
                Packet::new(Cmd::PageDirectory, encode_page_directory_payload())
            }
            Cmd::GetTableDirectory => {
                Packet::new(Cmd::TableDirectory, encode_table_directory_payload())
            }
            Cmd::GetPinDirectory => Packet::new(Cmd::PinDirectory, encode_pin_directory_payload()),
            Cmd::GetPinAssignments => Packet::new(
                Cmd::PinAssignments,
                encode_pin_assignments_payload(&self.pin_assignments),
            ),
            Cmd::ReadPage => match decode_page_request(&packet.payload) {
                Ok(page_id) => match self.store.read_page(page_id) {
                    Some(page) => Packet::new(Cmd::PageData, encode_page_payload(page_id, page)),
                    None => nack(RuntimeNackCode::InvalidPage, "invalid page id"),
                },
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad read-page payload"),
            },
            Cmd::WritePage => match decode_page_payload(&packet.payload) {
                Ok(page) => match self.store.write_page(page.page_id, &page.payload) {
                    Ok(()) => Packet::new(
                        Cmd::Ack,
                        encode_ack_payload(
                            page.page_id,
                            self.store.needs_burn(page.page_id).unwrap_or(false),
                        ),
                    ),
                    Err(_) => nack(RuntimeNackCode::StorageFailure, "page write failed"),
                },
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad write-page payload"),
            },
            Cmd::BurnPage => match decode_page_request(&packet.payload) {
                Ok(page_id) => match self.store.burn_page(page_id) {
                    Ok(()) => Packet::new(Cmd::Ack, encode_ack_payload(page_id, false)),
                    Err(_) => nack(RuntimeNackCode::StorageFailure, "page burn failed"),
                },
                Err(_) => nack(RuntimeNackCode::MalformedPayload, "bad burn-page payload"),
            },
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
    use crate::io::{EcuFunction, PinAssignmentRequest};
    use crate::protocol::{
        decode_ack_payload, decode_capabilities_payload, decode_identity_payload,
        decode_nack_payload, decode_page_payload, decode_pin_assignments_payload,
        decode_pin_directory_payload, encode_page_payload, encode_page_request, Cmd, Packet,
    };

    use super::FirmwareRuntime;

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
}
