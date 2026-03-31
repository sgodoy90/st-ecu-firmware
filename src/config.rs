use crate::contract::PageDirectoryEntry;
use crc::{Crc, CRC_32_ISO_HDLC};

static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

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
    PageDirectoryEntry {
        id: ConfigPage::BaseEngineFuelComm as u8,
        key: "base_engine_fuel_comm",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::TriggerIgnition as u8,
        key: "trigger_ignition",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::Sensors as u8,
        key: "sensors",
        byte_length: 1024,
    },
    PageDirectoryEntry {
        id: ConfigPage::PinAssignment as u8,
        key: "pin_assignment",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::IdleBoostVvt as u8,
        key: "idle_boost_vvt",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::LimitsKnock as u8,
        key: "limits_knock",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::AdvancedAirTorque as u8,
        key: "advanced_air_torque",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::ProtectionsThermal as u8,
        key: "protections_thermal",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::VehicleIntegration as u8,
        key: "vehicle_integration",
        byte_length: 512,
    },
    PageDirectoryEntry {
        id: ConfigPage::UiDefaults as u8,
        key: "ui_defaults",
        byte_length: 256,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigPageHeader {
    pub page_id: u8,
    pub schema_version: u16,
    pub payload_crc: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigPageStatus {
    pub page_id: u8,
    pub ram_crc: u32,
    pub flash_crc: u32,
    pub needs_burn: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    ram_pages: Vec<Vec<u8>>,
    flash_pages: Vec<Vec<u8>>,
}

impl ConfigStore {
    pub fn new_zeroed() -> Self {
        let pages = PAGE_DIRECTORY
            .iter()
            .map(|entry| vec![0u8; entry.byte_length as usize])
            .collect::<Vec<_>>();
        Self {
            ram_pages: pages.clone(),
            flash_pages: pages,
        }
    }

    pub fn read_page(&self, page_id: u8) -> Option<&[u8]> {
        self.ram_pages.get(page_id as usize).map(Vec::as_slice)
    }

    pub fn read_flash_page(&self, page_id: u8) -> Option<&[u8]> {
        self.flash_pages.get(page_id as usize).map(Vec::as_slice)
    }

    pub fn page_length(&self, page_id: u8) -> Option<usize> {
        self.ram_pages.get(page_id as usize).map(Vec::len)
    }

    pub fn write_page(&mut self, page_id: u8, data: &[u8]) -> Result<(), &'static str> {
        let page = self
            .ram_pages
            .get_mut(page_id as usize)
            .ok_or("invalid page id")?;
        if page.len() != data.len() {
            return Err("page length mismatch");
        }
        page.copy_from_slice(data);
        Ok(())
    }

    pub fn burn_page(&mut self, page_id: u8) -> Result<(), &'static str> {
        let ram = self
            .ram_pages
            .get(page_id as usize)
            .ok_or("invalid page id")?
            .clone();
        let flash = self
            .flash_pages
            .get_mut(page_id as usize)
            .ok_or("invalid page id")?;
        flash.copy_from_slice(&ram);
        Ok(())
    }

    pub fn burn_all_dirty(&mut self) -> usize {
        let dirty_pages = self
            .ram_pages
            .iter()
            .zip(self.flash_pages.iter())
            .enumerate()
            .filter_map(|(index, (ram, flash))| (ram != flash).then_some(index as u8))
            .collect::<Vec<_>>();

        for page_id in &dirty_pages {
            self.burn_page(*page_id)
                .expect("dirty page id must be valid");
        }

        dirty_pages.len()
    }

    pub fn restore_page_from_flash(&mut self, page_id: u8) -> Result<(), &'static str> {
        let flash = self
            .flash_pages
            .get(page_id as usize)
            .ok_or("invalid page id")?
            .clone();
        let ram = self
            .ram_pages
            .get_mut(page_id as usize)
            .ok_or("invalid page id")?;
        ram.copy_from_slice(&flash);
        Ok(())
    }

    pub fn page_header(&self, page_id: u8) -> Option<ConfigPageHeader> {
        let payload = self.read_page(page_id)?;
        Some(ConfigPageHeader {
            page_id,
            schema_version: crate::SCHEMA_VERSION,
            payload_crc: CRC32.checksum(payload),
        })
    }

    pub fn flash_page_crc(&self, page_id: u8) -> Option<u32> {
        self.flash_pages
            .get(page_id as usize)
            .map(|page| CRC32.checksum(page))
    }

    pub fn needs_burn(&self, page_id: u8) -> Option<bool> {
        let ram = self.ram_pages.get(page_id as usize)?;
        let flash = self.flash_pages.get(page_id as usize)?;
        Some(ram != flash)
    }

    pub fn page_status(&self, page_id: u8) -> Option<ConfigPageStatus> {
        let ram = self.ram_pages.get(page_id as usize)?;
        let flash = self.flash_pages.get(page_id as usize)?;
        let ram_crc = CRC32.checksum(ram);
        let flash_crc = CRC32.checksum(flash);
        Some(ConfigPageStatus {
            page_id,
            ram_crc,
            flash_crc,
            needs_burn: ram_crc != flash_crc || ram != flash,
        })
    }

    pub fn all_page_statuses(&self) -> Vec<ConfigPageStatus> {
        PAGE_DIRECTORY
            .iter()
            .filter_map(|page| self.page_status(page.id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigPage, ConfigStore};

    #[test]
    fn page_write_and_burn_roundtrip() {
        let mut store = ConfigStore::new_zeroed();
        let payload = vec![0xAB; 512];
        store
            .write_page(ConfigPage::BaseEngineFuelComm as u8, &payload)
            .unwrap();
        store
            .burn_page(ConfigPage::BaseEngineFuelComm as u8)
            .unwrap();

        assert_eq!(
            store
                .read_page(ConfigPage::BaseEngineFuelComm as u8)
                .unwrap(),
            payload.as_slice()
        );
        assert_eq!(
            store
                .page_header(ConfigPage::BaseEngineFuelComm as u8)
                .unwrap()
                .payload_crc,
            store
                .flash_page_crc(ConfigPage::BaseEngineFuelComm as u8)
                .unwrap(),
        );
    }

    #[test]
    fn needs_burn_tracks_ram_flash_divergence() {
        let mut store = ConfigStore::new_zeroed();
        let page_id = ConfigPage::TriggerIgnition as u8;

        assert_eq!(store.needs_burn(page_id), Some(false));

        let payload = vec![0x5A; store.page_length(page_id).unwrap()];
        store.write_page(page_id, &payload).unwrap();
        assert_eq!(store.needs_burn(page_id), Some(true));

        assert_ne!(
            store.read_page(page_id).unwrap(),
            store.read_flash_page(page_id).unwrap()
        );

        store.restore_page_from_flash(page_id).unwrap();
        assert_eq!(store.needs_burn(page_id), Some(false));
    }

    #[test]
    fn burn_all_dirty_only_burns_changed_pages() {
        let mut store = ConfigStore::new_zeroed();
        let page_a = ConfigPage::Sensors as u8;
        let page_b = ConfigPage::VehicleIntegration as u8;

        store
            .write_page(page_a, &vec![0x11; store.page_length(page_a).unwrap()])
            .unwrap();
        store
            .write_page(page_b, &vec![0x22; store.page_length(page_b).unwrap()])
            .unwrap();

        assert_eq!(store.burn_all_dirty(), 2);
        assert_eq!(store.needs_burn(page_a), Some(false));
        assert_eq!(store.needs_burn(page_b), Some(false));
        assert_eq!(store.burn_all_dirty(), 0);
    }

    #[test]
    fn page_status_reports_dirty_and_clean_crc_state() {
        let mut store = ConfigStore::new_zeroed();
        let page_id = ConfigPage::PinAssignment as u8;

        let clean = store.page_status(page_id).unwrap();
        assert!(!clean.needs_burn);
        assert_eq!(clean.ram_crc, clean.flash_crc);

        let payload = vec![0x33; store.page_length(page_id).unwrap()];
        store.write_page(page_id, &payload).unwrap();

        let dirty = store.page_status(page_id).unwrap();
        assert!(dirty.needs_burn);
        assert_ne!(dirty.ram_crc, dirty.flash_crc);
    }
}
