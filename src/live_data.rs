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

    pub fn encode_payload(&self) -> Vec<u8> {
        let mut payload = vec![0u8; LIVE_DATA_SIZE];
        payload[0..2].copy_from_slice(&self.schema_version.to_be_bytes());
        payload[2..4].copy_from_slice(&(self.byte_length as u16).to_be_bytes());
        payload
    }
}

#[cfg(test)]
mod tests {
    use super::{LiveDataFrame, LIVE_DATA_SIZE};

    #[test]
    fn payload_encoding_matches_live_data_size() {
        let payload = LiveDataFrame::current().encode_payload();
        assert_eq!(payload.len(), LIVE_DATA_SIZE);
        assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), 1);
    }
}
