/// STM32H743 HAL implementation — superset of F407.
/// Adds: HRTIM (217 ps), FDCAN (8 Mbps), 16-bit ADC, CORDIC, AES-256, True RNG, Backup SRAM.
///
/// Key H743 specs:
///   CPU:  Cortex-M7F @ 480 MHz, 64KB ITCM, 128KB DTCM
///   ADC:  16-bit, 3.6 MSPS (ADC1/ADC2/ADC3)
///   HRTIM: 4.608 GHz timer clock → 217 ps resolution
///   FDCAN: 2× CAN-FD @ up to 8 Mbps
///   CORDIC: hardware sin/cos/atan2 for VVT angle calc
///   AES-256-GCM: hardware crypto
///   True RNG: 48 Mbit/s hardware entropy
///   Backup SRAM: 4KB with VBAT retention

use super::common::*;

// ─── HRTIM (High-Resolution Timer) ───────────────────────────────────────────

/// H743-exclusive: 217 ps timer for injection/ignition scheduling and DBW H-bridge peak-and-hold.
pub struct H743Hrtim {
    pub scheduled_events: [(u64, u8); 16], // (ns_absolute, callback_id)
    pub event_count: usize,
    pub counter_ns: u64,
}

impl Default for H743Hrtim {
    fn default() -> Self {
        Self {
            scheduled_events: [(0, 0); 16],
            event_count: 0,
            counter_ns: 0,
        }
    }
}

impl H743Hrtim {
    /// Resolution: 1 tick = ~217 ps (1 / 4.608 GHz)
    pub const TICK_PS: u32 = 217;

    /// Schedule an event with sub-microsecond precision.
    pub fn schedule_precise(&mut self, abs_ns: u64, callback_id: u8) -> HalResult<()> {
        if self.event_count < 16 {
            self.scheduled_events[self.event_count] = (abs_ns, callback_id);
            self.event_count += 1;
            Ok(())
        } else {
            Err(HalError::PeripheralBusy)
        }
    }

    /// Set peak-and-hold for DBW H-bridge motor (reduces heat, improves precision).
    /// peak_duty: full current for first `peak_us` microseconds
    /// hold_duty: reduced holding current after peak
    pub fn set_peak_and_hold(&mut self, peak_duty: f32, hold_duty: f32, peak_us: u32) -> HalResult<()> {
        let _ = (peak_duty, hold_duty, peak_us); // on target → HRTIM compare units
        Ok(())
    }
}

impl HalTimer for H743Hrtim {
    fn ic_configure(&mut self, _channel: u8, _edge: CaptureEdge) -> HalResult<()> {
        Ok(())
    }

    fn ic_read_ns(&self, channel: u8) -> HalResult<u64> {
        if channel < 16 {
            Ok(self.scheduled_events[channel as usize].0)
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn oc_schedule_ns(&mut self, channel: u8, ns_from_now: u64, callback_id: u8) -> HalResult<()> {
        self.schedule_precise(self.counter_ns + ns_from_now, callback_id)?;
        let _ = channel;
        Ok(())
    }

    fn oc_cancel(&mut self, _channel: u8) {
        // On target: clear the HRTIM compare register
    }

    fn now_ns(&self) -> u64 {
        self.counter_ns
    }
}

// ─── ADC 16-bit ──────────────────────────────────────────────────────────────

pub struct H743Adc {
    pub raw: [u16; 20],
}

impl Default for H743Adc {
    fn default() -> Self {
        Self { raw: [32768u16; 20] }
    }
}

impl HalAdc for H743Adc {
    fn start_conversion(&mut self) -> HalResult<()> {
        Ok(())
    }

    fn read_mv(&self, channel: u8) -> HalResult<u16> {
        let raw = self.read_raw(channel)?;
        // H743 Vref = 3300 mV, 16-bit (65535 max)
        Ok(((raw as u32 * 3300) / 65535) as u16)
    }

    fn read_raw(&self, channel: u8) -> HalResult<u16> {
        if (channel as usize) < self.raw.len() {
            Ok(self.raw[channel as usize])
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn resolution_bits(&self) -> u8 {
        16
    }
}

// ─── CAN-FD (FDCAN 8 Mbps) ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanFdFrame {
    pub id: u32,
    pub extended: bool,
    /// DLC 0–15 (CAN-FD supports up to 64 bytes)
    pub dlc: u8,
    pub data: [u8; 64],
    /// True if data phase uses faster bitrate
    pub brs: bool,
}

impl Default for CanFdFrame {
    fn default() -> Self {
        Self {
            id: 0,
            extended: false,
            dlc: 0,
            data: [0u8; 64],
            brs: false,
        }
    }
}

pub struct H743Fdcan {
    pub tx_queue: [Option<CanFdFrame>; 16],
    pub rx_queue: [Option<CanFdFrame>; 16],
    pub tx_head: usize,
    pub rx_head: usize,
    pub data_bitrate_mbps: u8,
}

impl Default for H743Fdcan {
    fn default() -> Self {
        Self {
            tx_queue: [None; 16],
            rx_queue: [None; 16],
            tx_head: 0,
            rx_head: 0,
            data_bitrate_mbps: 8,
        }
    }
}

impl H743Fdcan {
    pub fn transmit_fd(&mut self, frame: &CanFdFrame) -> HalResult<()> {
        if self.tx_head < 16 {
            self.tx_queue[self.tx_head] = Some(*frame);
            self.tx_head += 1;
            Ok(())
        } else {
            Err(HalError::PeripheralBusy)
        }
    }

    /// Transmit ST binary protocol packet over CAN-FD (up to 64 bytes per frame).
    pub fn transmit_st_protocol(&mut self, packet: &[u8]) -> HalResult<()> {
        let mut offset = 0;
        while offset < packet.len() {
            let chunk_len = (packet.len() - offset).min(64);
            let mut frame = CanFdFrame::default();
            frame.id = 0x7E0; // ST ECU protocol CAN ID
            frame.extended = false;
            frame.dlc = chunk_len as u8;
            frame.brs = true; // use 8 Mbps data phase
            frame.data[..chunk_len].copy_from_slice(&packet[offset..offset + chunk_len]);
            self.transmit_fd(&frame)?;
            offset += chunk_len;
        }
        Ok(())
    }
}

// Also implement HalCan for compatibility with common CAN trait
impl HalCan for H743Fdcan {
    fn transmit(&mut self, frame: &CanFrame) -> HalResult<()> {
        let mut fd_frame = CanFdFrame::default();
        fd_frame.id = frame.id;
        fd_frame.extended = frame.extended;
        fd_frame.dlc = frame.dlc;
        fd_frame.data[..8].copy_from_slice(&frame.data);
        self.transmit_fd(&fd_frame)
    }

    fn receive(&mut self) -> HalResult<Option<CanFrame>> {
        if self.rx_head > 0 {
            let fd = self.rx_queue[self.rx_head - 1].take();
            self.rx_head -= 1;
            if let Some(f) = fd {
                let mut frame = CanFrame {
                    id: f.id,
                    extended: f.extended,
                    dlc: f.dlc.min(8),
                    data: [0u8; 8],
                };
                frame.data.copy_from_slice(&f.data[..8]);
                Ok(Some(frame))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn set_filter(&mut self, _id: u32, _mask: u32) -> HalResult<()> {
        Ok(())
    }

    fn bitrate_kbps(&self) -> u32 {
        (self.data_bitrate_mbps as u32) * 1000
    }
}

// ─── CORDIC (hardware sin/cos for VVT angle) ─────────────────────────────────

pub struct H743Cordic;

impl H743Cordic {
    /// Hardware sin/cos computation (faster than software on M7).
    /// angle_rad: input angle in radians
    /// Returns (sin, cos)
    pub fn sincos(&self, angle_rad: f32) -> (f32, f32) {
        // On target: write to CORDIC_WDATA, read CORDIC_RDATA
        // Here: use libm for host testing
        (angle_rad.sin(), angle_rad.cos())
    }

    /// Hardware atan2 for cam angle calculation.
    pub fn atan2(&self, y: f32, x: f32) -> f32 {
        y.atan2(x)
    }
}

// ─── AES-256 Tune Encryption ─────────────────────────────────────────────────

pub struct H743Aes {
    /// 256-bit key (32 bytes)
    pub key: [u8; 32],
    pub key_loaded: bool,
}

impl Default for H743Aes {
    fn default() -> Self {
        Self {
            key: [0u8; 32],
            key_loaded: false,
        }
    }
}

impl H743Aes {
    /// Load a 256-bit encryption key.
    pub fn load_key(&mut self, key: &[u8; 32]) -> HalResult<()> {
        self.key.copy_from_slice(key);
        self.key_loaded = true;
        Ok(())
    }

    /// Encrypt a config page in-place (AES-256-GCM on target, XOR stub for host tests).
    pub fn encrypt_page(&self, data: &mut [u8]) -> HalResult<()> {
        if !self.key_loaded {
            return Err(HalError::NotSupported);
        }
        // Stub: XOR with first key byte (real implementation uses hardware AES)
        for byte in data.iter_mut() {
            *byte ^= self.key[0];
        }
        Ok(())
    }

    /// Decrypt a config page in-place.
    pub fn decrypt_page(&self, data: &mut [u8]) -> HalResult<()> {
        self.encrypt_page(data) // XOR is symmetric (stub only)
    }
}

// ─── True RNG ────────────────────────────────────────────────────────────────

pub struct H743Rng {
    pub entropy_counter: u32,
}

impl Default for H743Rng {
    fn default() -> Self {
        Self { entropy_counter: 0xDEADBEEF }
    }
}

impl H743Rng {
    /// Generate a 32-bit random number.
    /// On target: reads TRNG_DR register (48 Mbit/s hardware entropy).
    pub fn random_u32(&mut self) -> HalResult<u32> {
        // Stub: simple LCG for host tests
        self.entropy_counter = self.entropy_counter
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        Ok(self.entropy_counter)
    }

    /// Generate a unique ECU ID (32 bytes from True RNG).
    pub fn generate_ecu_id(&mut self) -> HalResult<[u8; 32]> {
        let mut id = [0u8; 32];
        for chunk in id.chunks_exact_mut(4) {
            let r = self.random_u32()?;
            chunk.copy_from_slice(&r.to_be_bytes());
        }
        Ok(id)
    }
}

// ─── Backup SRAM (4KB, VBAT retained) ────────────────────────────────────────

pub struct H743BackupRam {
    pub data: [u8; 4096],
}

impl Default for H743BackupRam {
    fn default() -> Self {
        Self { data: [0u8; 4096] }
    }
}

impl HalBackupRam for H743BackupRam {
    fn write(&mut self, offset: usize, data: &[u8]) -> HalResult<()> {
        if offset + data.len() <= self.data.len() {
            self.data[offset..offset + data.len()].copy_from_slice(data);
            Ok(())
        } else {
            Err(HalError::OverUnderflow)
        }
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> HalResult<()> {
        if offset + buf.len() <= self.data.len() {
            buf.copy_from_slice(&self.data[offset..offset + buf.len()]);
            Ok(())
        } else {
            Err(HalError::OverUnderflow)
        }
    }

    fn capacity_bytes(&self) -> usize {
        4096
    }
}

// ─── Flash (dual-bank) ────────────────────────────────────────────────────────

pub struct H743Flash {
    pub pages: [[u8; 1024]; 10],
}

impl Default for H743Flash {
    fn default() -> Self {
        Self { pages: [[0xFFu8; 1024]; 10] }
    }
}

impl HalFlash for H743Flash {
    fn erase_page(&mut self, page_id: u8) -> HalResult<()> {
        if (page_id as usize) < self.pages.len() {
            self.pages[page_id as usize] = [0xFFu8; 1024];
            Ok(())
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn write_page(&mut self, page_id: u8, data: &[u8]) -> HalResult<()> {
        let idx = page_id as usize;
        if idx < self.pages.len() && data.len() <= 1024 {
            self.pages[idx][..data.len()].copy_from_slice(data);
            Ok(())
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn read_page(&self, page_id: u8, buf: &mut [u8]) -> HalResult<()> {
        let idx = page_id as usize;
        if idx < self.pages.len() && buf.len() <= 1024 {
            buf.copy_from_slice(&self.pages[idx][..buf.len()]);
            Ok(())
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn supports_dual_bank(&self) -> bool {
        true // H743 has 2× 1MB banks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h743_adc_resolution() {
        let adc = H743Adc::default();
        assert_eq!(adc.resolution_bits(), 16);
    }

    #[test]
    fn h743_fdcan_bitrate_8mbps() {
        let can = H743Fdcan::default();
        assert_eq!(can.bitrate_kbps(), 8000);
    }

    #[test]
    fn h743_hrtim_tick_precision() {
        assert_eq!(H743Hrtim::TICK_PS, 217);
    }

    #[test]
    fn h743_flash_dual_bank() {
        let flash = H743Flash::default();
        assert!(flash.supports_dual_bank());
    }

    #[test]
    fn h743_backup_ram_4kb() {
        let bram = H743BackupRam::default();
        assert_eq!(bram.capacity_bytes(), 4096);
    }

    #[test]
    fn h743_backup_ram_write_read_roundtrip() {
        let mut bram = H743BackupRam::default();
        let data = [0xAA, 0xBB, 0xCC, 0xDD];
        bram.write(100, &data).unwrap();
        let mut buf = [0u8; 4];
        bram.read(100, &mut buf).unwrap();
        assert_eq!(buf, data);
    }

    #[test]
    fn h743_rng_generates_nonzero() {
        let mut rng = H743Rng::default();
        let v = rng.random_u32().unwrap();
        assert_ne!(v, 0);
    }

    #[test]
    fn h743_rng_ecu_id_32_bytes() {
        let mut rng = H743Rng::default();
        let id = rng.generate_ecu_id().unwrap();
        assert_eq!(id.len(), 32);
        // Not all zeros
        assert!(id.iter().any(|&b| b != 0));
    }

    #[test]
    fn h743_aes_encrypt_decrypt_roundtrip() {
        let mut aes = H743Aes::default();
        let key = [0x42u8; 32];
        aes.load_key(&key).unwrap();
        let original = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let mut data = original;
        aes.encrypt_page(&mut data).unwrap();
        assert_ne!(data, original);
        aes.decrypt_page(&mut data).unwrap();
        assert_eq!(data, original);
    }

    #[test]
    fn h743_cordic_sincos_identity() {
        let cordic = H743Cordic;
        let (s, c) = cordic.sincos(0.0);
        assert!((s - 0.0).abs() < 1e-6);
        assert!((c - 1.0).abs() < 1e-6);
    }

    #[test]
    fn canfd_st_protocol_transmit() {
        let mut can = H743Fdcan::default();
        let packet = [0u8; 100]; // > 64 bytes, splits into 2 frames
        can.transmit_st_protocol(&packet).unwrap();
        assert_eq!(can.tx_head, 2);
    }
}
