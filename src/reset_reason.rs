#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetReason {
    PowerOn,
    Software,
    Watchdog,
    Brownout,
    Unknown,
}
