use crate::config::ConfigStore;
use crate::contract::{base_capabilities, Capability, FirmwareIdentity};
use crate::io::{
    apply_assignment_overrides, default_pin_assignments, deserialize_assignments_from_page,
    serialize_assignments_to_page, validate_assignment_set, AssignmentError, PinAssignmentRequest,
    ResolvedPinAssignment,
};
use crate::live_data::LiveDataFrame;
use crate::network::{headless_network_profile, NetworkProfile};
use crate::protocol::{
    decode_page_payload, decode_page_request, encode_ack_payload, encode_capabilities_payload,
    encode_identity_payload, encode_nack_payload, encode_network_profile_payload,
    encode_output_test_directory_payload, encode_page_directory_payload, encode_page_payload,
    encode_page_statuses_payload, encode_pin_assignments_payload, encode_pin_directory_payload,
    encode_table_directory_payload, encode_table_metadata_payload, Cmd, OutputTestDirectoryEntry,
    Packet,
};
use crate::ConfigPage;

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
    pub network_profile: &'static NetworkProfile,
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

impl FirmwareRuntime {
    pub fn new(identity: FirmwareIdentity, simulator: bool) -> Self {
        let mut store = ConfigStore::new_zeroed();
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
        Self {
            identity,
            transport: TransportCapabilities::default(),
            store,
            capabilities: base_capabilities(simulator),
            pin_assignments,
            network_profile: headless_network_profile(),
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
            Cmd::GetTableMetadata => {
                Packet::new(Cmd::TableMetadata, encode_table_metadata_payload())
            }
            Cmd::GetPinDirectory => Packet::new(Cmd::PinDirectory, encode_pin_directory_payload()),
            Cmd::GetPinAssignments => Packet::new(
                Cmd::PinAssignments,
                encode_pin_assignments_payload(&self.pin_assignments),
            ),
            Cmd::GetOutputTestDirectory => Packet::new(
                Cmd::OutputTestDirectory,
                encode_output_test_directory_payload(&OUTPUT_TEST_DIRECTORY),
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
    use crate::io::{
        apply_assignment_overrides, deserialize_assignments_from_page,
        serialize_assignments_to_page, EcuFunction, PinAssignmentRequest,
    };
    use crate::network::{MessageClass, ProductTrack, TransportLinkKind};
    use crate::protocol::{
        decode_ack_payload, decode_capabilities_payload, decode_identity_payload,
        decode_nack_payload, decode_network_profile_payload, decode_output_test_directory_payload,
        decode_page_payload, decode_page_statuses_payload, decode_pin_assignments_payload,
        decode_pin_directory_payload, encode_page_payload, encode_page_request, Cmd, Packet,
    };
    use crate::ConfigPage;

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
