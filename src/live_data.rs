pub const LIVE_DATA_SIZE: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveDataFrame {
    pub schema_version: u16,
    pub byte_length: usize,
}

impl LiveDataFrame {
    pub const fn current() -> Self {
        Self {
            schema_version: 1,
            byte_length: LIVE_DATA_SIZE,
        }
    }
}
