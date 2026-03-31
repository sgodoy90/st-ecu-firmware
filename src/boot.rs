#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMode {
    Application,
    Bootloader,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootPolicy {
    pub watchdog_required: bool,
    pub rollback_supported: bool,
}

impl Default for BootPolicy {
    fn default() -> Self {
        Self {
            watchdog_required: true,
            rollback_supported: true,
        }
    }
}
