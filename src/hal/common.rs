/// Common HAL traits — implemented by both F407 and H743 targets.
/// Each trait maps to one peripheral category. All methods are
/// unit-safe (SI units in function signatures, scaled in hardware impl).

// ─── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalError {
    Timeout,
    BusError,
    OverUnderflow,
    NotSupported,
    InvalidPin,
    PeripheralBusy,
}

pub type HalResult<T> = Result<T, HalError>;

// ─── Timer / Input Capture ───────────────────────────────────────────────────

/// General purpose timer for input capture (trigger wheel) and output compare (injection/ignition).
pub trait HalTimer {
    /// Configure input capture on a channel (rising/falling/both edge).
    fn ic_configure(&mut self, channel: u8, edge: CaptureEdge) -> HalResult<()>;

    /// Read the latest input-capture timestamp in nanoseconds.
    fn ic_read_ns(&self, channel: u8) -> HalResult<u64>;

    /// Schedule an output-compare event at `ns_from_now` nanoseconds from now.
    /// `callback_id` identifies which output event fired when the ISR triggers.
    fn oc_schedule_ns(&mut self, channel: u8, ns_from_now: u64, callback_id: u8) -> HalResult<()>;

    /// Cancel a pending OC event.
    fn oc_cancel(&mut self, channel: u8);

    /// Current counter value in nanoseconds (free-running).
    fn now_ns(&self) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEdge {
    Rising,
    Falling,
    Both,
}

// ─── ADC ─────────────────────────────────────────────────────────────────────

pub trait HalAdc {
    /// Start a DMA-backed conversion on all configured channels.
    fn start_conversion(&mut self) -> HalResult<()>;

    /// Read the latest converted value for `channel` in millivolts.
    fn read_mv(&self, channel: u8) -> HalResult<u16>;

    /// Read raw counts (12-bit on F407, 16-bit on H743).
    fn read_raw(&self, channel: u8) -> HalResult<u16>;

    /// ADC resolution in bits.
    fn resolution_bits(&self) -> u8;
}

// ─── CAN ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanFrame {
    pub id: u32,
    pub extended: bool,
    pub dlc: u8,
    pub data: [u8; 8],
}

pub trait HalCan {
    fn transmit(&mut self, frame: &CanFrame) -> HalResult<()>;
    fn receive(&mut self) -> HalResult<Option<CanFrame>>;
    fn set_filter(&mut self, id: u32, mask: u32) -> HalResult<()>;
    fn bitrate_kbps(&self) -> u32;
}

// ─── UART / USB-CDC ──────────────────────────────────────────────────────────

pub trait HalUart {
    fn write_bytes(&mut self, data: &[u8]) -> HalResult<usize>;
    fn read_bytes(&mut self, buf: &mut [u8]) -> HalResult<usize>;
    fn bytes_available(&self) -> usize;
    fn flush(&mut self) -> HalResult<()>;
}

// ─── SPI ─────────────────────────────────────────────────────────────────────

pub trait HalSpi {
    fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> HalResult<()>;
    fn write(&mut self, data: &[u8]) -> HalResult<()>;
}

// ─── GPIO ────────────────────────────────────────────────────────────────────

pub trait HalGpio {
    fn set_high(&mut self, pin: u8) -> HalResult<()>;
    fn set_low(&mut self, pin: u8) -> HalResult<()>;
    fn read(&self, pin: u8) -> HalResult<bool>;
    fn set_pwm_duty(&mut self, pin: u8, duty_0_to_1: f32) -> HalResult<()>;
}

// ─── Flash (config page storage) ─────────────────────────────────────────────

pub trait HalFlash {
    /// Erase a page (sector). Returns Err if write-protected or ECC fault.
    fn erase_page(&mut self, page_id: u8) -> HalResult<()>;

    /// Write `data` to the given flash page offset. Must be page-erased first.
    fn write_page(&mut self, page_id: u8, data: &[u8]) -> HalResult<()>;

    /// Read from flash page into `buf`.
    fn read_page(&self, page_id: u8, buf: &mut [u8]) -> HalResult<()>;

    /// True if this target supports dual-bank live OTA (H743 only).
    fn supports_dual_bank(&self) -> bool;
}

// ─── Watchdog ────────────────────────────────────────────────────────────────

pub trait HalWatchdog {
    /// Kick the independent watchdog. Must be called within the configured window.
    fn kick(&mut self);

    /// Configure IWDG timeout in milliseconds (call once at startup).
    fn configure_ms(&mut self, timeout_ms: u32) -> HalResult<()>;
}

// ─── Clock / Timestamp ───────────────────────────────────────────────────────

pub trait HalClock {
    /// Monotonic millisecond counter since boot.
    fn millis(&self) -> u32;

    /// Monotonic nanosecond counter (uses DWT cycle counter if available).
    fn nanos(&self) -> u64;

    /// CPU cycle counter (for profiling).
    fn cycles(&self) -> u32;

    /// CPU frequency in Hz.
    fn cpu_hz(&self) -> u32;
}

// ─── Backup SRAM (H743 only, stub on F407) ───────────────────────────────────

pub trait HalBackupRam {
    /// Write `data` to backup SRAM at `offset`. Survives power-off with VBAT.
    fn write(&mut self, offset: usize, data: &[u8]) -> HalResult<()>;

    /// Read from backup SRAM.
    fn read(&self, offset: usize, buf: &mut [u8]) -> HalResult<()>;

    /// Available backup SRAM size in bytes (0 if not supported).
    fn capacity_bytes(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hal_error_is_copy() {
        let e = HalError::Timeout;
        let _e2 = e; // copy
        assert_eq!(e, HalError::Timeout);
    }

    #[test]
    fn can_frame_default_dlc() {
        let f = CanFrame {
            id: 0x100,
            extended: false,
            dlc: 8,
            data: [0u8; 8],
        };
        assert_eq!(f.dlc, 8);
    }
}
