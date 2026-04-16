/// STM32F407 HAL implementation stubs.
/// These compile to no-ops for host-side testing.
/// On target hardware, each method maps to STM32 LL/HAL register writes.
///
/// Key F407 specs:
///   CPU:  Cortex-M4F @ 168 MHz
///   ADC:  12-bit, 2.4 MSPS
///   CAN:  2× bxCAN @ 1 Mbps
///   USB:  FS 12 Mbps
///   Timer precision: ~1 µs (84 MHz APB2 clock)
///   No HRTIM, no CORDIC, no CAN-FD, no Backup SRAM

use super::common::*;

// ─── Timer ────────────────────────────────────────────────────────────────────

pub struct F407Timer {
    pub channel_timestamps: [u64; 4],
    pub now_counter: u64,
}

impl Default for F407Timer {
    fn default() -> Self {
        Self {
            channel_timestamps: [0u64; 4],
            now_counter: 0,
        }
    }
}

impl HalTimer for F407Timer {
    fn ic_configure(&mut self, _channel: u8, _edge: CaptureEdge) -> HalResult<()> {
        Ok(())
    }

    fn ic_read_ns(&self, channel: u8) -> HalResult<u64> {
        if channel < 4 {
            Ok(self.channel_timestamps[channel as usize])
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn oc_schedule_ns(&mut self, _channel: u8, _ns_from_now: u64, _callback_id: u8) -> HalResult<()> {
        Ok(())
    }

    fn oc_cancel(&mut self, _channel: u8) {}

    fn now_ns(&self) -> u64 {
        self.now_counter
    }
}

// ─── ADC ─────────────────────────────────────────────────────────────────────

pub struct F407Adc {
    /// Simulated raw 12-bit readings
    pub raw: [u16; 16],
}

impl Default for F407Adc {
    fn default() -> Self {
        Self { raw: [2048u16; 16] }
    }
}

impl HalAdc for F407Adc {
    fn start_conversion(&mut self) -> HalResult<()> {
        Ok(())
    }

    fn read_mv(&self, channel: u8) -> HalResult<u16> {
        let raw = self.read_raw(channel)?;
        // F407 Vref = 3300 mV, 12-bit (4095 max)
        Ok(((raw as u32 * 3300) / 4095) as u16)
    }

    fn read_raw(&self, channel: u8) -> HalResult<u16> {
        if (channel as usize) < self.raw.len() {
            Ok(self.raw[channel as usize])
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn resolution_bits(&self) -> u8 {
        12
    }
}

// ─── CAN (bxCAN 1 Mbps) ──────────────────────────────────────────────────────

pub struct F407Can {
    pub tx_queue: [Option<CanFrame>; 8],
    pub rx_queue: [Option<CanFrame>; 8],
    pub tx_head: usize,
    pub rx_head: usize,
}

impl Default for F407Can {
    fn default() -> Self {
        Self {
            tx_queue: [None; 8],
            rx_queue: [None; 8],
            tx_head: 0,
            rx_head: 0,
        }
    }
}

impl HalCan for F407Can {
    fn transmit(&mut self, frame: &CanFrame) -> HalResult<()> {
        if self.tx_head < 8 {
            self.tx_queue[self.tx_head] = Some(*frame);
            self.tx_head += 1;
            Ok(())
        } else {
            Err(HalError::PeripheralBusy)
        }
    }

    fn receive(&mut self) -> HalResult<Option<CanFrame>> {
        if self.rx_head > 0 {
            let frame = self.rx_queue[self.rx_head - 1].take();
            self.rx_head -= 1;
            Ok(frame)
        } else {
            Ok(None)
        }
    }

    fn set_filter(&mut self, _id: u32, _mask: u32) -> HalResult<()> {
        Ok(())
    }

    fn bitrate_kbps(&self) -> u32 {
        1000
    }
}

// ─── UART (USB-CDC bridge) ────────────────────────────────────────────────────

pub struct F407Uart {
    pub tx_buf: [u8; 4096],
    pub tx_len: usize,
    pub rx_buf: [u8; 4096],
    pub rx_len: usize,
}

impl Default for F407Uart {
    fn default() -> Self {
        Self {
            tx_buf: [0u8; 4096],
            tx_len: 0,
            rx_buf: [0u8; 4096],
            rx_len: 0,
        }
    }
}

impl HalUart for F407Uart {
    fn write_bytes(&mut self, data: &[u8]) -> HalResult<usize> {
        let n = data.len().min(self.tx_buf.len() - self.tx_len);
        self.tx_buf[self.tx_len..self.tx_len + n].copy_from_slice(&data[..n]);
        self.tx_len += n;
        Ok(n)
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> HalResult<usize> {
        let n = self.rx_len.min(buf.len());
        buf[..n].copy_from_slice(&self.rx_buf[..n]);
        self.rx_len -= n;
        Ok(n)
    }

    fn bytes_available(&self) -> usize {
        self.rx_len
    }

    fn flush(&mut self) -> HalResult<()> {
        self.tx_len = 0;
        Ok(())
    }
}

// ─── Flash ────────────────────────────────────────────────────────────────────

pub struct F407Flash {
    pub pages: [[u8; 512]; 10],
}

impl Default for F407Flash {
    fn default() -> Self {
        Self {
            pages: [[0xFFu8; 512]; 10],
        }
    }
}

impl HalFlash for F407Flash {
    fn erase_page(&mut self, page_id: u8) -> HalResult<()> {
        if (page_id as usize) < self.pages.len() {
            self.pages[page_id as usize] = [0xFFu8; 512];
            Ok(())
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn write_page(&mut self, page_id: u8, data: &[u8]) -> HalResult<()> {
        let idx = page_id as usize;
        if idx < self.pages.len() && data.len() <= 512 {
            self.pages[idx][..data.len()].copy_from_slice(data);
            Ok(())
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn read_page(&self, page_id: u8, buf: &mut [u8]) -> HalResult<()> {
        let idx = page_id as usize;
        if idx < self.pages.len() && buf.len() <= 512 {
            buf.copy_from_slice(&self.pages[idx][..buf.len()]);
            Ok(())
        } else {
            Err(HalError::InvalidPin)
        }
    }

    fn supports_dual_bank(&self) -> bool {
        false // F407 = single bank
    }
}

// ─── Watchdog ─────────────────────────────────────────────────────────────────

pub struct F407Watchdog {
    pub kick_count: u32,
}

impl Default for F407Watchdog {
    fn default() -> Self {
        Self { kick_count: 0 }
    }
}

impl HalWatchdog for F407Watchdog {
    fn kick(&mut self) {
        self.kick_count += 1;
    }

    fn configure_ms(&mut self, _timeout_ms: u32) -> HalResult<()> {
        Ok(())
    }
}

// ─── Backup SRAM (not present on F407) ───────────────────────────────────────

pub struct F407BackupRam;

impl HalBackupRam for F407BackupRam {
    fn write(&mut self, _offset: usize, _data: &[u8]) -> HalResult<()> {
        Err(HalError::NotSupported)
    }

    fn read(&self, _offset: usize, _buf: &mut [u8]) -> HalResult<()> {
        Err(HalError::NotSupported)
    }

    fn capacity_bytes(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f407_adc_resolution() {
        let adc = F407Adc::default();
        assert_eq!(adc.resolution_bits(), 12);
    }

    #[test]
    fn f407_can_bitrate() {
        let can = F407Can::default();
        assert_eq!(can.bitrate_kbps(), 1000);
    }

    #[test]
    fn f407_flash_no_dual_bank() {
        let flash = F407Flash::default();
        assert!(!flash.supports_dual_bank());
    }

    #[test]
    fn f407_backup_ram_not_supported() {
        let bram = F407BackupRam;
        assert_eq!(bram.capacity_bytes(), 0);
        assert_eq!(bram.read(0, &mut []).unwrap_err(), HalError::NotSupported);
    }

    #[test]
    fn f407_uart_write_read() {
        let mut uart = F407Uart::default();
        uart.write_bytes(b"HELLO").unwrap();
        assert_eq!(uart.tx_len, 5);
    }

    #[test]
    fn f407_watchdog_kick_increments() {
        let mut wd = F407Watchdog::default();
        wd.kick();
        wd.kick();
        assert_eq!(wd.kick_count, 2);
    }
}
