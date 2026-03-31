#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductTrack {
    HeadlessEcu,
    DisplayIntegratedVcu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductTrackParseError {
    pub code: u8,
}

impl ProductTrack {
    pub const fn code(self) -> u8 {
        match self {
            Self::HeadlessEcu => 0x01,
            Self::DisplayIntegratedVcu => 0x02,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::HeadlessEcu => "headless_ecu",
            Self::DisplayIntegratedVcu => "display_integrated_vcu",
        }
    }
}

impl TryFrom<u8> for ProductTrack {
    type Error = ProductTrackParseError;

    fn try_from(value: u8) -> Result<Self, ProductTrackParseError> {
        match value {
            0x01 => Ok(Self::HeadlessEcu),
            0x02 => Ok(Self::DisplayIntegratedVcu),
            _ => Err(ProductTrackParseError { code: value }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportLinkKind {
    UsbSerial,
    CanFdPrimary,
    CanFdSecondary,
    WifiBridge,
    Bluetooth,
    LocalDisplayLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportLinkParseError {
    pub code: u8,
}

impl TransportLinkKind {
    pub const fn code(self) -> u8 {
        match self {
            Self::UsbSerial => 0x01,
            Self::CanFdPrimary => 0x02,
            Self::CanFdSecondary => 0x03,
            Self::WifiBridge => 0x04,
            Self::Bluetooth => 0x05,
            Self::LocalDisplayLink => 0x06,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::UsbSerial => "usb_serial",
            Self::CanFdPrimary => "can_fd_primary",
            Self::CanFdSecondary => "can_fd_secondary",
            Self::WifiBridge => "wifi_bridge",
            Self::Bluetooth => "bluetooth",
            Self::LocalDisplayLink => "local_display_link",
        }
    }
}

impl TryFrom<u8> for TransportLinkKind {
    type Error = TransportLinkParseError;

    fn try_from(value: u8) -> Result<Self, TransportLinkParseError> {
        match value {
            0x01 => Ok(Self::UsbSerial),
            0x02 => Ok(Self::CanFdPrimary),
            0x03 => Ok(Self::CanFdSecondary),
            0x04 => Ok(Self::WifiBridge),
            0x05 => Ok(Self::Bluetooth),
            0x06 => Ok(Self::LocalDisplayLink),
            _ => Err(TransportLinkParseError { code: value }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageClass {
    Identity,
    CalibrationPages,
    CalibrationTables,
    PageStatuses,
    PinMetadata,
    LiveData,
    DashboardFrames,
    Datalog,
    Diagnostics,
    FirmwareUpdate,
    IoExpansion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageClassParseError {
    pub code: u8,
}

impl MessageClass {
    pub const fn code(self) -> u8 {
        match self {
            Self::Identity => 0x01,
            Self::CalibrationPages => 0x02,
            Self::CalibrationTables => 0x03,
            Self::PageStatuses => 0x04,
            Self::PinMetadata => 0x05,
            Self::LiveData => 0x06,
            Self::DashboardFrames => 0x07,
            Self::Datalog => 0x08,
            Self::Diagnostics => 0x09,
            Self::FirmwareUpdate => 0x0A,
            Self::IoExpansion => 0x0B,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::CalibrationPages => "calibration_pages",
            Self::CalibrationTables => "calibration_tables",
            Self::PageStatuses => "page_statuses",
            Self::PinMetadata => "pin_metadata",
            Self::LiveData => "live_data",
            Self::DashboardFrames => "dashboard_frames",
            Self::Datalog => "datalog",
            Self::Diagnostics => "diagnostics",
            Self::FirmwareUpdate => "firmware_update",
            Self::IoExpansion => "io_expansion",
        }
    }
}

impl TryFrom<u8> for MessageClass {
    type Error = MessageClassParseError;

    fn try_from(value: u8) -> Result<Self, MessageClassParseError> {
        match value {
            0x01 => Ok(Self::Identity),
            0x02 => Ok(Self::CalibrationPages),
            0x03 => Ok(Self::CalibrationTables),
            0x04 => Ok(Self::PageStatuses),
            0x05 => Ok(Self::PinMetadata),
            0x06 => Ok(Self::LiveData),
            0x07 => Ok(Self::DashboardFrames),
            0x08 => Ok(Self::Datalog),
            0x09 => Ok(Self::Diagnostics),
            0x0A => Ok(Self::FirmwareUpdate),
            0x0B => Ok(Self::IoExpansion),
            _ => Err(MessageClassParseError { code: value }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkNodeKind {
    EcuCore,
    DesktopApp,
    MobileApp,
    DisplayHmi,
    PowerIoModule,
    Keypad,
    WidebandModule,
}

impl NetworkNodeKind {
    pub const fn key(self) -> &'static str {
        match self {
            Self::EcuCore => "ecu_core",
            Self::DesktopApp => "desktop_app",
            Self::MobileApp => "mobile_app",
            Self::DisplayHmi => "display_hmi",
            Self::PowerIoModule => "power_io_module",
            Self::Keypad => "keypad",
            Self::WidebandModule => "wideband_module",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkContract {
    pub kind: TransportLinkKind,
    pub realtime_safe: bool,
    pub firmware_update_allowed: bool,
    pub classes: &'static [MessageClass],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkProfile {
    pub key: &'static str,
    pub product_track: ProductTrack,
    pub multi_master_can: bool,
    pub nodes: &'static [NetworkNodeKind],
    pub links: &'static [LinkContract],
}

const USB_CLASSES: &[MessageClass] = &[
    MessageClass::Identity,
    MessageClass::CalibrationPages,
    MessageClass::CalibrationTables,
    MessageClass::PageStatuses,
    MessageClass::PinMetadata,
    MessageClass::LiveData,
    MessageClass::Datalog,
    MessageClass::Diagnostics,
    MessageClass::FirmwareUpdate,
];

const CAN_CLASSES: &[MessageClass] = &[
    MessageClass::Identity,
    MessageClass::PageStatuses,
    MessageClass::PinMetadata,
    MessageClass::LiveData,
    MessageClass::DashboardFrames,
    MessageClass::Diagnostics,
    MessageClass::IoExpansion,
];

const WIFI_CLASSES: &[MessageClass] = &[
    MessageClass::Identity,
    MessageClass::CalibrationPages,
    MessageClass::CalibrationTables,
    MessageClass::PageStatuses,
    MessageClass::PinMetadata,
    MessageClass::LiveData,
    MessageClass::Datalog,
    MessageClass::Diagnostics,
];

const DISPLAY_LINK_CLASSES: &[MessageClass] = &[
    MessageClass::Identity,
    MessageClass::PageStatuses,
    MessageClass::PinMetadata,
    MessageClass::LiveData,
    MessageClass::DashboardFrames,
    MessageClass::Datalog,
    MessageClass::Diagnostics,
    MessageClass::IoExpansion,
];

const HEADLESS_NODES: &[NetworkNodeKind] = &[
    NetworkNodeKind::EcuCore,
    NetworkNodeKind::DesktopApp,
    NetworkNodeKind::MobileApp,
    NetworkNodeKind::PowerIoModule,
    NetworkNodeKind::WidebandModule,
];

const DISPLAY_NODES: &[NetworkNodeKind] = &[
    NetworkNodeKind::EcuCore,
    NetworkNodeKind::DesktopApp,
    NetworkNodeKind::MobileApp,
    NetworkNodeKind::DisplayHmi,
    NetworkNodeKind::PowerIoModule,
    NetworkNodeKind::Keypad,
    NetworkNodeKind::WidebandModule,
];

pub const HEADLESS_LINKS: [LinkContract; 3] = [
    LinkContract {
        kind: TransportLinkKind::UsbSerial,
        realtime_safe: false,
        firmware_update_allowed: true,
        classes: USB_CLASSES,
    },
    LinkContract {
        kind: TransportLinkKind::CanFdPrimary,
        realtime_safe: true,
        firmware_update_allowed: false,
        classes: CAN_CLASSES,
    },
    LinkContract {
        kind: TransportLinkKind::WifiBridge,
        realtime_safe: false,
        firmware_update_allowed: false,
        classes: WIFI_CLASSES,
    },
];

pub const DISPLAY_LINKS: [LinkContract; 5] = [
    LinkContract {
        kind: TransportLinkKind::UsbSerial,
        realtime_safe: false,
        firmware_update_allowed: true,
        classes: USB_CLASSES,
    },
    LinkContract {
        kind: TransportLinkKind::CanFdPrimary,
        realtime_safe: true,
        firmware_update_allowed: false,
        classes: CAN_CLASSES,
    },
    LinkContract {
        kind: TransportLinkKind::CanFdSecondary,
        realtime_safe: true,
        firmware_update_allowed: false,
        classes: CAN_CLASSES,
    },
    LinkContract {
        kind: TransportLinkKind::WifiBridge,
        realtime_safe: false,
        firmware_update_allowed: false,
        classes: WIFI_CLASSES,
    },
    LinkContract {
        kind: TransportLinkKind::LocalDisplayLink,
        realtime_safe: true,
        firmware_update_allowed: false,
        classes: DISPLAY_LINK_CLASSES,
    },
];

pub const HEADLESS_PROFILE: NetworkProfile = NetworkProfile {
    key: "headless-ecu-v1",
    product_track: ProductTrack::HeadlessEcu,
    multi_master_can: false,
    nodes: HEADLESS_NODES,
    links: &HEADLESS_LINKS,
};

pub const DISPLAY_PROFILE: NetworkProfile = NetworkProfile {
    key: "display-vcu-v1",
    product_track: ProductTrack::DisplayIntegratedVcu,
    multi_master_can: true,
    nodes: DISPLAY_NODES,
    links: &DISPLAY_LINKS,
};

pub fn headless_network_profile() -> &'static NetworkProfile {
    &HEADLESS_PROFILE
}

pub fn display_network_profile() -> &'static NetworkProfile {
    &DISPLAY_PROFILE
}

pub fn supports_message(
    profile: &NetworkProfile,
    link: TransportLinkKind,
    class: MessageClass,
) -> bool {
    profile
        .links
        .iter()
        .find(|contract| contract.kind == link)
        .is_some_and(|contract| contract.classes.contains(&class))
}

pub fn preferred_links(profile: &NetworkProfile, class: MessageClass) -> Vec<TransportLinkKind> {
    profile
        .links
        .iter()
        .filter(|contract| contract.classes.contains(&class))
        .map(|contract| contract.kind)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        display_network_profile, headless_network_profile, preferred_links, supports_message,
        MessageClass, ProductTrack, TransportLinkKind,
    };

    #[test]
    fn headless_profile_blocks_firmware_update_on_can() {
        let profile = headless_network_profile();
        assert_eq!(profile.product_track, ProductTrack::HeadlessEcu);
        assert!(!supports_message(
            profile,
            TransportLinkKind::CanFdPrimary,
            MessageClass::FirmwareUpdate
        ));
        assert!(supports_message(
            profile,
            TransportLinkKind::UsbSerial,
            MessageClass::FirmwareUpdate
        ));
    }

    #[test]
    fn display_profile_enables_multimaster_can_and_local_display_link() {
        let profile = display_network_profile();
        assert!(profile.multi_master_can);
        assert!(supports_message(
            profile,
            TransportLinkKind::LocalDisplayLink,
            MessageClass::DashboardFrames
        ));
        assert!(supports_message(
            profile,
            TransportLinkKind::CanFdPrimary,
            MessageClass::IoExpansion
        ));
    }

    #[test]
    fn preferred_links_prioritize_realtime_channels() {
        let profile = display_network_profile();
        let links = preferred_links(profile, MessageClass::DashboardFrames);
        assert!(links.contains(&TransportLinkKind::CanFdPrimary));
        assert!(links.contains(&TransportLinkKind::LocalDisplayLink));
        assert!(!links.contains(&TransportLinkKind::UsbSerial));
    }
}
