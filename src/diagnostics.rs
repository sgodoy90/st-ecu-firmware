#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtcSeverity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtcCode {
    pub code: &'static str,
    pub severity: DtcSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreezeFrameHeader {
    pub rev_counter: u32,
    pub reason_id: u16,
}
