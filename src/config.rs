use crate::contract::{PageDirectoryEntry, CONFIG_FORMAT_VERSION};
use crc::{Crc, CRC_32_ISO_HDLC};

static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub const CONFIG_IMAGE_MAGIC: [u8; 4] = *b"STCF";
pub const MIN_SUPPORTED_CONFIG_FORMAT_VERSION: u8 = CONFIG_FORMAT_VERSION;
const STORED_PAGE_PREFIX_LEN: usize = 18;
const STORED_PAGE_HEADER_LEN: usize = 22;

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
    pub format_version: u8,
    pub generation: u32,
    pub payload_length: u16,
    pub payload_crc: u32,
    pub image_crc: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigPageStatus {
    pub page_id: u8,
    pub ram_crc: u32,
    pub flash_crc: u32,
    pub needs_burn: bool,
    pub flash_generation: u32,
    pub flash_valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigImageError {
    MalformedImage,
    InvalidMagic,
    UnsupportedFormat(u8),
    ImageCrcMismatch,
    PayloadCrcMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigMigrationError {
    UnsupportedSchemaVersion(u16),
    UnsupportedFormatVersion(u8),
    MissingSchemaMigrator { from: u16, to: u16 },
    MissingFormatMigrator { from: u8, to: u8 },
    PayloadLengthMismatch { expected: usize, actual: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigMigrationReport {
    pub page_id: u8,
    pub from_schema_version: u16,
    pub to_schema_version: u16,
    pub from_format_version: u8,
    pub to_format_version: u8,
    pub schema_steps: u16,
    pub format_steps: u8,
    pub migrated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigStoreError {
    InvalidPageId,
    PageIdMismatch { expected: u8, actual: u8 },
    PageLengthMismatch,
    NoValidFlashImage,
    FlashImageInvalid(ConfigImageError),
    MigrationFailed(ConfigMigrationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredPageImage {
    header: ConfigPageHeader,
    payload: Vec<u8>,
}

#[derive(Debug, Clone)]
struct FlashPageSlot {
    raw_image: Vec<u8>,
    committed: Option<StoredPageImage>,
    erase_cycles: u32,
}

impl FlashPageSlot {
    fn zeroed(page_id: u8, page_length: usize) -> Self {
        let payload = vec![0u8; page_length];
        let raw_image = encode_stored_page_image(page_id, 0, &payload);
        let committed = Some(
            decode_stored_page_image(&raw_image)
                .expect("factory config image must decode after encode"),
        );
        Self {
            raw_image,
            committed,
            erase_cycles: 0,
        }
    }

    fn replace_raw_image(&mut self, raw_image: Vec<u8>) {
        self.committed = decode_stored_page_image(&raw_image).ok();
        self.raw_image = raw_image;
    }
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    ram_pages: Vec<Vec<u8>>,
    flash_slots: Vec<FlashPageSlot>,
}

impl ConfigStore {
    pub fn new_zeroed() -> Self {
        let ram_pages = PAGE_DIRECTORY
            .iter()
            .map(|entry| vec![0u8; entry.byte_length as usize])
            .collect::<Vec<_>>();
        let flash_slots = PAGE_DIRECTORY
            .iter()
            .map(|entry| FlashPageSlot::zeroed(entry.id, entry.byte_length as usize))
            .collect::<Vec<_>>();
        Self {
            ram_pages,
            flash_slots,
        }
    }

    pub fn read_page(&self, page_id: u8) -> Option<&[u8]> {
        self.ram_pages.get(page_id as usize).map(Vec::as_slice)
    }

    pub fn read_flash_page(&self, page_id: u8) -> Option<&[u8]> {
        self.flash_slots.get(page_id as usize).and_then(|slot| {
            slot.committed
                .as_ref()
                .map(|image| image.payload.as_slice())
        })
    }

    pub fn page_length(&self, page_id: u8) -> Option<usize> {
        self.ram_pages.get(page_id as usize).map(Vec::len)
    }

    pub fn import_persisted_page_image(
        &mut self,
        page_id: u8,
        raw_image: &[u8],
    ) -> Result<ConfigMigrationReport, ConfigStoreError> {
        let expected_length = self
            .page_length(page_id)
            .ok_or(ConfigStoreError::InvalidPageId)?;

        let decoded =
            decode_stored_page_image(raw_image).map_err(ConfigStoreError::FlashImageInvalid)?;
        if decoded.header.page_id != page_id {
            return Err(ConfigStoreError::PageIdMismatch {
                expected: page_id,
                actual: decoded.header.page_id,
            });
        }

        let from_schema_version = decoded.header.schema_version;
        let from_format_version = decoded.header.format_version;
        let generation = decoded.header.generation;

        let (migrated_payload, schema_steps, format_steps) =
            migrate_payload_to_current_versions(page_id, decoded, expected_length)
                .map_err(ConfigStoreError::MigrationFailed)?;

        let migrated_raw = encode_stored_page_image_with_versions(
            page_id,
            generation,
            crate::SCHEMA_VERSION,
            CONFIG_FORMAT_VERSION,
            &migrated_payload,
        );

        let slot = self
            .flash_slots
            .get_mut(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?;
        slot.replace_raw_image(migrated_raw);
        if slot.committed.is_none() {
            return Err(ConfigStoreError::NoValidFlashImage);
        }

        let ram = self
            .ram_pages
            .get_mut(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?;
        ram.copy_from_slice(&migrated_payload);

        Ok(ConfigMigrationReport {
            page_id,
            from_schema_version,
            to_schema_version: crate::SCHEMA_VERSION,
            from_format_version,
            to_format_version: CONFIG_FORMAT_VERSION,
            schema_steps,
            format_steps,
            migrated: schema_steps > 0
                || format_steps > 0
                || from_schema_version != crate::SCHEMA_VERSION
                || from_format_version != CONFIG_FORMAT_VERSION,
        })
    }

    pub fn write_page(&mut self, page_id: u8, data: &[u8]) -> Result<(), ConfigStoreError> {
        let page = self
            .ram_pages
            .get_mut(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?;
        if page.len() != data.len() {
            return Err(ConfigStoreError::PageLengthMismatch);
        }
        page.copy_from_slice(data);
        Ok(())
    }

    pub fn burn_page(&mut self, page_id: u8) -> Result<(), ConfigStoreError> {
        let ram = self
            .ram_pages
            .get(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?
            .clone();
        let slot = self
            .flash_slots
            .get_mut(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?;
        let next_generation = slot
            .committed
            .as_ref()
            .map_or(0, |image| image.header.generation.saturating_add(1));
        slot.replace_raw_image(encode_stored_page_image(page_id, next_generation, &ram));
        slot.erase_cycles = slot.erase_cycles.saturating_add(1);
        if slot.committed.is_none() {
            return Err(ConfigStoreError::NoValidFlashImage);
        }
        Ok(())
    }

    pub fn burn_all_dirty(&mut self) -> usize {
        let dirty_pages = PAGE_DIRECTORY
            .iter()
            .filter_map(|page| self.needs_burn(page.id).unwrap_or(false).then_some(page.id))
            .collect::<Vec<_>>();

        for page_id in &dirty_pages {
            self.burn_page(*page_id)
                .expect("dirty page id must burn successfully");
        }

        dirty_pages.len()
    }

    pub fn restore_page_from_flash(&mut self, page_id: u8) -> Result<(), ConfigStoreError> {
        let payload = self
            .flash_slots
            .get(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?
            .committed
            .as_ref()
            .ok_or(ConfigStoreError::NoValidFlashImage)?
            .payload
            .clone();
        let ram = self
            .ram_pages
            .get_mut(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?;
        ram.copy_from_slice(&payload);
        Ok(())
    }

    pub fn page_header(&self, page_id: u8) -> Option<ConfigPageHeader> {
        let payload = self.read_page(page_id)?;
        let flash_generation = self.flash_page_generation(page_id).unwrap_or(0);
        let next_generation =
            if self.flash_page_valid(page_id).unwrap_or(false) && !self.needs_burn(page_id)? {
                flash_generation
            } else {
                flash_generation
                    .saturating_add(u32::from(self.flash_page_valid(page_id).unwrap_or(false)))
            };
        Some(build_page_header(page_id, next_generation, payload))
    }

    pub fn flash_page_header(&self, page_id: u8) -> Option<ConfigPageHeader> {
        self.flash_slots
            .get(page_id as usize)
            .and_then(|slot| slot.committed.as_ref().map(|image| image.header))
    }

    pub fn flash_page_crc(&self, page_id: u8) -> Option<u32> {
        self.flash_page_header(page_id)
            .map(|header| header.payload_crc)
    }

    pub fn flash_page_generation(&self, page_id: u8) -> Option<u32> {
        self.flash_page_header(page_id)
            .map(|header| header.generation)
    }

    pub fn flash_page_valid(&self, page_id: u8) -> Option<bool> {
        self.flash_slots
            .get(page_id as usize)
            .map(|slot| slot.committed.is_some())
    }

    pub fn flash_erase_cycles(&self, page_id: u8) -> Option<u32> {
        self.flash_slots
            .get(page_id as usize)
            .map(|slot| slot.erase_cycles)
    }

    pub fn needs_burn(&self, page_id: u8) -> Option<bool> {
        let ram = self.ram_pages.get(page_id as usize)?;
        let slot = self.flash_slots.get(page_id as usize)?;
        Some(match &slot.committed {
            Some(image) => image.payload.as_slice() != ram.as_slice(),
            None => true,
        })
    }

    pub fn page_status(&self, page_id: u8) -> Option<ConfigPageStatus> {
        let ram = self.ram_pages.get(page_id as usize)?;
        let slot = self.flash_slots.get(page_id as usize)?;
        let ram_crc = CRC32.checksum(ram);
        let (flash_crc, flash_generation, flash_valid, needs_burn) = match &slot.committed {
            Some(image) => (
                image.header.payload_crc,
                image.header.generation,
                true,
                image.payload.as_slice() != ram.as_slice(),
            ),
            None => (0, 0, false, true),
        };
        Some(ConfigPageStatus {
            page_id,
            ram_crc,
            flash_crc,
            needs_burn,
            flash_generation,
            flash_valid,
        })
    }

    pub fn all_page_statuses(&self) -> Vec<ConfigPageStatus> {
        PAGE_DIRECTORY
            .iter()
            .filter_map(|page| self.page_status(page.id))
            .collect()
    }

    #[cfg(test)]
    fn replace_flash_raw_image(
        &mut self,
        page_id: u8,
        raw_image: Vec<u8>,
    ) -> Result<(), ConfigStoreError> {
        let slot = self
            .flash_slots
            .get_mut(page_id as usize)
            .ok_or(ConfigStoreError::InvalidPageId)?;
        slot.replace_raw_image(raw_image);
        Ok(())
    }
}

fn build_page_header(page_id: u8, generation: u32, payload: &[u8]) -> ConfigPageHeader {
    build_page_header_with_versions(
        page_id,
        generation,
        crate::SCHEMA_VERSION,
        CONFIG_FORMAT_VERSION,
        payload,
    )
}

fn build_page_header_with_versions(
    page_id: u8,
    generation: u32,
    schema_version: u16,
    format_version: u8,
    payload: &[u8],
) -> ConfigPageHeader {
    let payload_length =
        u16::try_from(payload.len()).expect("config page payload length must fit u16");
    let payload_crc = CRC32.checksum(payload);
    let image_crc = calculate_image_crc(
        format_version,
        schema_version,
        page_id,
        generation,
        payload_length,
        payload_crc,
        payload,
    );
    ConfigPageHeader {
        page_id,
        schema_version,
        format_version,
        generation,
        payload_length,
        payload_crc,
        image_crc,
    }
}

fn encode_stored_page_image(page_id: u8, generation: u32, payload: &[u8]) -> Vec<u8> {
    encode_stored_page_image_with_versions(
        page_id,
        generation,
        crate::SCHEMA_VERSION,
        CONFIG_FORMAT_VERSION,
        payload,
    )
}

fn encode_stored_page_image_with_versions(
    page_id: u8,
    generation: u32,
    schema_version: u16,
    format_version: u8,
    payload: &[u8],
) -> Vec<u8> {
    let header = build_page_header_with_versions(
        page_id,
        generation,
        schema_version,
        format_version,
        payload,
    );
    let mut out = encode_image_prefix(&header);
    out.extend_from_slice(&header.image_crc.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn decode_stored_page_image(raw_image: &[u8]) -> Result<StoredPageImage, ConfigImageError> {
    if raw_image.len() < STORED_PAGE_HEADER_LEN {
        return Err(ConfigImageError::MalformedImage);
    }
    if raw_image[0..4] != CONFIG_IMAGE_MAGIC {
        return Err(ConfigImageError::InvalidMagic);
    }

    let format_version = raw_image[4];
    if format_version < MIN_SUPPORTED_CONFIG_FORMAT_VERSION
        || format_version > CONFIG_FORMAT_VERSION
    {
        return Err(ConfigImageError::UnsupportedFormat(format_version));
    }

    let page_id = raw_image[5];
    let schema_version = u16::from_be_bytes([raw_image[6], raw_image[7]]);
    let generation = u32::from_be_bytes([raw_image[8], raw_image[9], raw_image[10], raw_image[11]]);
    let payload_length = u16::from_be_bytes([raw_image[12], raw_image[13]]);
    let payload_crc =
        u32::from_be_bytes([raw_image[14], raw_image[15], raw_image[16], raw_image[17]]);
    let image_crc =
        u32::from_be_bytes([raw_image[18], raw_image[19], raw_image[20], raw_image[21]]);

    if raw_image.len() != STORED_PAGE_HEADER_LEN + payload_length as usize {
        return Err(ConfigImageError::MalformedImage);
    }

    let payload = raw_image[STORED_PAGE_HEADER_LEN..].to_vec();
    if CRC32.checksum(&payload) != payload_crc {
        return Err(ConfigImageError::PayloadCrcMismatch);
    }

    let expected_image_crc = calculate_image_crc(
        format_version,
        schema_version,
        page_id,
        generation,
        payload_length,
        payload_crc,
        &payload,
    );
    if expected_image_crc != image_crc {
        return Err(ConfigImageError::ImageCrcMismatch);
    }

    Ok(StoredPageImage {
        header: ConfigPageHeader {
            page_id,
            schema_version,
            format_version,
            generation,
            payload_length,
            payload_crc,
            image_crc,
        },
        payload,
    })
}

type SchemaMigrator = fn(u8, &[u8]) -> Result<Vec<u8>, ConfigMigrationError>;
type FormatMigrator = fn(u8, &[u8]) -> Result<Vec<u8>, ConfigMigrationError>;

fn schema_migrator_for(_from_version: u16) -> Option<SchemaMigrator> {
    None
}

fn format_migrator_for(_from_version: u8) -> Option<FormatMigrator> {
    None
}

fn migrate_payload_to_current_versions(
    page_id: u8,
    image: StoredPageImage,
    expected_length: usize,
) -> Result<(Vec<u8>, u16, u8), ConfigMigrationError> {
    let mut schema_version = image.header.schema_version;
    let mut format_version = image.header.format_version;
    let mut payload = image.payload;
    let mut schema_steps = 0u16;
    let mut format_steps = 0u8;

    if payload.len() != expected_length {
        return Err(ConfigMigrationError::PayloadLengthMismatch {
            expected: expected_length,
            actual: payload.len(),
        });
    }

    if schema_version > crate::SCHEMA_VERSION {
        return Err(ConfigMigrationError::UnsupportedSchemaVersion(
            schema_version,
        ));
    }
    while schema_version < crate::SCHEMA_VERSION {
        let to = schema_version.saturating_add(1);
        let migrator = schema_migrator_for(schema_version).ok_or(
            ConfigMigrationError::MissingSchemaMigrator {
                from: schema_version,
                to,
            },
        )?;
        payload = migrator(page_id, &payload)?;
        schema_version = to;
        schema_steps = schema_steps.saturating_add(1);
        if payload.len() != expected_length {
            return Err(ConfigMigrationError::PayloadLengthMismatch {
                expected: expected_length,
                actual: payload.len(),
            });
        }
    }

    if format_version < MIN_SUPPORTED_CONFIG_FORMAT_VERSION {
        return Err(ConfigMigrationError::UnsupportedFormatVersion(
            format_version,
        ));
    }
    if format_version > CONFIG_FORMAT_VERSION {
        return Err(ConfigMigrationError::UnsupportedFormatVersion(
            format_version,
        ));
    }
    while format_version < CONFIG_FORMAT_VERSION {
        let to = format_version.saturating_add(1);
        let migrator = format_migrator_for(format_version).ok_or(
            ConfigMigrationError::MissingFormatMigrator {
                from: format_version,
                to,
            },
        )?;
        payload = migrator(page_id, &payload)?;
        format_version = to;
        format_steps = format_steps.saturating_add(1);
        if payload.len() != expected_length {
            return Err(ConfigMigrationError::PayloadLengthMismatch {
                expected: expected_length,
                actual: payload.len(),
            });
        }
    }

    Ok((payload, schema_steps, format_steps))
}

fn encode_image_prefix(header: &ConfigPageHeader) -> Vec<u8> {
    let mut out = Vec::with_capacity(STORED_PAGE_PREFIX_LEN);
    out.extend_from_slice(&CONFIG_IMAGE_MAGIC);
    out.push(header.format_version);
    out.push(header.page_id);
    out.extend_from_slice(&header.schema_version.to_be_bytes());
    out.extend_from_slice(&header.generation.to_be_bytes());
    out.extend_from_slice(&header.payload_length.to_be_bytes());
    out.extend_from_slice(&header.payload_crc.to_be_bytes());
    out
}

fn calculate_image_crc(
    format_version: u8,
    schema_version: u16,
    page_id: u8,
    generation: u32,
    payload_length: u16,
    payload_crc: u32,
    payload: &[u8],
) -> u32 {
    let mut encoded = Vec::with_capacity(STORED_PAGE_PREFIX_LEN + payload.len());
    encoded.extend_from_slice(&CONFIG_IMAGE_MAGIC);
    encoded.push(format_version);
    encoded.push(page_id);
    encoded.extend_from_slice(&schema_version.to_be_bytes());
    encoded.extend_from_slice(&generation.to_be_bytes());
    encoded.extend_from_slice(&payload_length.to_be_bytes());
    encoded.extend_from_slice(&payload_crc.to_be_bytes());
    encoded.extend_from_slice(payload);
    CRC32.checksum(&encoded)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_stored_page_image, encode_stored_page_image, encode_stored_page_image_with_versions,
        ConfigImageError, ConfigMigrationError, ConfigPage, ConfigStore, ConfigStoreError,
        CONFIG_FORMAT_VERSION,
    };

    #[test]
    fn page_write_and_burn_roundtrip() {
        let mut store = ConfigStore::new_zeroed();
        let page_id = ConfigPage::BaseEngineFuelComm as u8;
        let payload = vec![0xAB; 512];
        store.write_page(page_id, &payload).unwrap();
        store.burn_page(page_id).unwrap();

        let staged_header = store.page_header(page_id).unwrap();
        let flash_header = store.flash_page_header(page_id).unwrap();

        assert_eq!(store.read_page(page_id).unwrap(), payload.as_slice());
        assert_eq!(store.read_flash_page(page_id).unwrap(), payload.as_slice());
        assert_eq!(staged_header.payload_crc, flash_header.payload_crc);
        assert_eq!(flash_header.generation, 1);
        assert_eq!(store.flash_erase_cycles(page_id), Some(1));
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
        assert_eq!(store.flash_page_generation(page_a), Some(1));
        assert_eq!(store.flash_page_generation(page_b), Some(1));
        assert_eq!(store.burn_all_dirty(), 0);
    }

    #[test]
    fn page_status_reports_dirty_and_clean_crc_state() {
        let mut store = ConfigStore::new_zeroed();
        let page_id = ConfigPage::PinAssignment as u8;

        let clean = store.page_status(page_id).unwrap();
        assert!(!clean.needs_burn);
        assert_eq!(clean.ram_crc, clean.flash_crc);
        assert!(clean.flash_valid);
        assert_eq!(clean.flash_generation, 0);

        let payload = vec![0x33; store.page_length(page_id).unwrap()];
        store.write_page(page_id, &payload).unwrap();

        let dirty = store.page_status(page_id).unwrap();
        assert!(dirty.needs_burn);
        assert_ne!(dirty.ram_crc, dirty.flash_crc);
        assert!(dirty.flash_valid);
    }

    #[test]
    fn stored_page_image_detects_crc_corruption() {
        let mut raw_image = encode_stored_page_image(ConfigPage::Sensors as u8, 7, &[1, 2, 3, 4]);
        let last_index = raw_image.len() - 1;
        raw_image[last_index] ^= 0xFF;

        assert_eq!(
            decode_stored_page_image(&raw_image),
            Err(ConfigImageError::PayloadCrcMismatch)
        );
    }

    #[test]
    fn stored_page_image_crc_accepts_legacy_schema_headers() {
        let legacy_schema_version = crate::SCHEMA_VERSION.saturating_sub(1);
        let raw_image = encode_stored_page_image_with_versions(
            ConfigPage::Sensors as u8,
            5,
            legacy_schema_version,
            CONFIG_FORMAT_VERSION,
            &[0x10, 0x20, 0x30, 0x40],
        );

        let decoded =
            decode_stored_page_image(&raw_image).expect("legacy schema image must decode");
        assert_eq!(decoded.header.schema_version, legacy_schema_version);
        assert_eq!(decoded.payload, vec![0x10, 0x20, 0x30, 0x40]);
    }

    #[test]
    fn invalid_flash_image_marks_page_dirty_until_reburned() {
        let mut store = ConfigStore::new_zeroed();
        let page_id = ConfigPage::IdleBoostVvt as u8;

        store
            .replace_flash_raw_image(page_id, vec![0x00; 8])
            .expect("page id must be valid");

        let corrupted = store.page_status(page_id).unwrap();
        assert!(!corrupted.flash_valid);
        assert!(corrupted.needs_burn);
        assert_eq!(
            store.restore_page_from_flash(page_id),
            Err(super::ConfigStoreError::NoValidFlashImage)
        );

        let payload = vec![0x42; store.page_length(page_id).unwrap()];
        store.write_page(page_id, &payload).unwrap();
        store.burn_page(page_id).unwrap();

        let restored = store.page_status(page_id).unwrap();
        assert!(restored.flash_valid);
        assert!(!restored.needs_burn);
        assert_eq!(store.read_flash_page(page_id).unwrap(), payload.as_slice());
    }

    #[test]
    fn import_persisted_image_restores_ram_and_flash_state() {
        let mut store = ConfigStore::new_zeroed();
        let page_id = ConfigPage::BaseEngineFuelComm as u8;
        let payload = vec![0xA6; store.page_length(page_id).unwrap()];
        let raw_image = encode_stored_page_image_with_versions(
            page_id,
            9,
            crate::SCHEMA_VERSION,
            CONFIG_FORMAT_VERSION,
            &payload,
        );

        let report = store
            .import_persisted_page_image(page_id, &raw_image)
            .expect("import should succeed for same schema/format");

        assert_eq!(report.from_schema_version, crate::SCHEMA_VERSION);
        assert_eq!(report.to_schema_version, crate::SCHEMA_VERSION);
        assert_eq!(report.from_format_version, CONFIG_FORMAT_VERSION);
        assert_eq!(report.to_format_version, CONFIG_FORMAT_VERSION);
        assert_eq!(report.schema_steps, 0);
        assert_eq!(report.format_steps, 0);
        assert!(!report.migrated);
        assert_eq!(store.read_page(page_id).unwrap(), payload.as_slice());
        assert_eq!(store.read_flash_page(page_id).unwrap(), payload.as_slice());
        assert_eq!(store.flash_page_generation(page_id), Some(9));
    }

    #[test]
    fn import_persisted_image_rejects_missing_schema_migrator() {
        let mut store = ConfigStore::new_zeroed();
        let page_id = ConfigPage::Sensors as u8;
        let legacy_schema = crate::SCHEMA_VERSION.saturating_sub(1);
        let payload = vec![0x55; store.page_length(page_id).unwrap()];
        let raw_image = encode_stored_page_image_with_versions(
            page_id,
            3,
            legacy_schema,
            CONFIG_FORMAT_VERSION,
            &payload,
        );

        let result = store.import_persisted_page_image(page_id, &raw_image);
        assert_eq!(
            result,
            Err(ConfigStoreError::MigrationFailed(
                ConfigMigrationError::MissingSchemaMigrator {
                    from: legacy_schema,
                    to: legacy_schema.saturating_add(1),
                },
            ))
        );
    }

    #[test]
    fn import_persisted_image_rejects_page_id_mismatch() {
        let mut store = ConfigStore::new_zeroed();
        let source_page_id = ConfigPage::Sensors as u8;
        let target_page_id = ConfigPage::TriggerIgnition as u8;
        let raw_image = encode_stored_page_image_with_versions(
            source_page_id,
            1,
            crate::SCHEMA_VERSION,
            CONFIG_FORMAT_VERSION,
            &vec![0x11; store.page_length(source_page_id).unwrap()],
        );

        assert_eq!(
            store.import_persisted_page_image(target_page_id, &raw_image),
            Err(ConfigStoreError::PageIdMismatch {
                expected: target_page_id,
                actual: source_page_id,
            }),
        );
    }
}
