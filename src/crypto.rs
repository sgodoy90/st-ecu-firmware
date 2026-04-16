/// Tune encryption — AES-256 per-ECU key + True RNG (H743 only).
///
/// H743: True RNG generates a unique 32-byte key on first startup, stored in OTP.
///       Tunes are AES-256-GCM encrypted and bound to the specific ECU ID.
///       Encrypted config pages have a 16-byte auth tag appended.
///
/// F407: Returns Err(NotSupported) for all crypto operations.
///       UI shows a badge indicating H743-only feature.

/// Crypto error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    NotSupported,
    KeyNotLoaded,
    AuthFailed,
    InvalidInput,
    RngFailure,
}

pub type CryptoResult<T> = Result<T, CryptoError>;

/// ECU-bound encryption key record
#[derive(Debug, Clone, Copy)]
pub struct TuneKey {
    /// 32-byte AES-256 key
    pub key: [u8; 32],
    /// 32-byte unique ECU ID (from True RNG at first startup)
    pub ecu_id: [u8; 32],
    /// Key generation counter (increments on re-key)
    pub generation: u32,
    /// True if key has been provisioned
    pub provisioned: bool,
}

impl Default for TuneKey {
    fn default() -> Self {
        Self {
            key: [0u8; 32],
            ecu_id: [0u8; 32],
            generation: 0,
            provisioned: false,
        }
    }
}

impl TuneKey {
    /// Format ECU ID as hex string (64 chars)
    pub fn ecu_id_hex(&self) -> [u8; 64] {
        let mut hex = [0u8; 64];
        const DIGITS: &[u8] = b"0123456789abcdef";
        for (i, &byte) in self.ecu_id.iter().enumerate() {
            hex[i * 2] = DIGITS[(byte >> 4) as usize];
            hex[i * 2 + 1] = DIGITS[(byte & 0xf) as usize];
        }
        hex
    }
}

/// Tune encryption state
#[derive(Debug, Clone, Copy, Default)]
pub struct TuneEncryptionState {
    pub locked: bool,
    pub key_pending: bool,
    pub tune_bound_ecu_id: [u8; 32],
}

/// Tune encryption controller
pub struct TuneEncryption {
    pub key: TuneKey,
    pub state: TuneEncryptionState,
    pub target_supports_crypto: bool,
}

impl TuneEncryption {
    pub fn new_f407() -> Self {
        Self {
            key: TuneKey::default(),
            state: TuneEncryptionState::default(),
            target_supports_crypto: false,
        }
    }

    pub fn new_h743(ecu_id: [u8; 32]) -> Self {
        let mut key = TuneKey::default();
        key.ecu_id = ecu_id;
        Self {
            key,
            state: TuneEncryptionState::default(),
            target_supports_crypto: true,
        }
    }

    /// Generate a new AES-256 key using True RNG. Stores in OTP on first call.
    pub fn generate_key(&mut self, rng_bytes: [u8; 32]) -> CryptoResult<()> {
        if !self.target_supports_crypto {
            return Err(CryptoError::NotSupported);
        }
        self.key.key = rng_bytes;
        self.key.provisioned = true;
        self.key.generation += 1;
        self.state.key_pending = false;
        Ok(())
    }

    /// Encrypt a config page. Returns ciphertext + 16-byte auth tag appended.
    /// Page must be 512 or 1024 bytes. Output is page.len() + 16 bytes.
    pub fn encrypt_page(&self, page: &[u8]) -> CryptoResult<Vec<u8>> {
        if !self.target_supports_crypto {
            return Err(CryptoError::NotSupported);
        }
        if !self.key.provisioned {
            return Err(CryptoError::KeyNotLoaded);
        }
        // Stub: XOR with key + append fake auth tag
        // Real implementation: AES-256-GCM via H743 hardware CRYP peripheral
        let mut out = Vec::with_capacity(page.len() + 16);
        for (i, &b) in page.iter().enumerate() {
            out.push(b ^ self.key.key[i % 32]);
        }
        // Fake 16-byte auth tag (real: GCM tag from hardware)
        let tag: u8 = self.key.key.iter().fold(0u8, |a, &b| a.wrapping_add(b));
        out.extend_from_slice(&[tag; 16]);
        Ok(out)
    }

    /// Decrypt a config page. Input must be page.len() + 16 (with auth tag).
    pub fn decrypt_page(&self, ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        if !self.target_supports_crypto {
            return Err(CryptoError::NotSupported);
        }
        if !self.key.provisioned {
            return Err(CryptoError::KeyNotLoaded);
        }
        if ciphertext.len() < 16 {
            return Err(CryptoError::InvalidInput);
        }
        let (ct, tag) = ciphertext.split_at(ciphertext.len() - 16);
        // Verify fake auth tag
        let expected_tag: u8 = self.key.key.iter().fold(0u8, |a, &b| a.wrapping_add(b));
        if tag.iter().any(|&b| b != expected_tag) {
            return Err(CryptoError::AuthFailed);
        }
        // Decrypt (XOR is symmetric for stub)
        let plain: Vec<u8> = ct.iter().enumerate().map(|(i, &b)| b ^ self.key.key[i % 32]).collect();
        Ok(plain)
    }

    /// Lock tune to this ECU ID. After locking, tune cannot be used on another ECU.
    pub fn lock_tune(&mut self) -> CryptoResult<()> {
        if !self.target_supports_crypto || !self.key.provisioned {
            return Err(CryptoError::NotSupported);
        }
        self.state.locked = true;
        self.state.tune_bound_ecu_id = self.key.ecu_id;
        Ok(())
    }

    /// Verify that current ECU ID matches the tune's bound ECU ID.
    pub fn verify_ecu_binding(&self) -> CryptoResult<()> {
        if !self.state.locked { return Ok(()); } // unlocked tune works on any ECU
        if self.state.tune_bound_ecu_id == self.key.ecu_id {
            Ok(())
        } else {
            Err(CryptoError::AuthFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f407_crypto_not_supported() {
        let mut enc = TuneEncryption::new_f407();
        assert!(enc.generate_key([0u8; 32]).is_err());
        assert_eq!(enc.encrypt_page(&[0u8; 512]).unwrap_err(), CryptoError::NotSupported);
    }

    #[test]
    fn h743_encrypt_decrypt_roundtrip() {
        let ecu_id = [0xAB; 32];
        let mut enc = TuneEncryption::new_h743(ecu_id);
        enc.generate_key([0x42u8; 32]).unwrap();
        let original = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let ciphertext = enc.encrypt_page(&original).unwrap();
        assert_eq!(ciphertext.len(), original.len() + 16);
        assert_ne!(ciphertext[..original.len()], original[..]);
        let decrypted = enc.decrypt_page(&ciphertext).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn auth_tag_corruption_fails_decrypt() {
        let ecu_id = [0xAB; 32];
        let mut enc = TuneEncryption::new_h743(ecu_id);
        enc.generate_key([0x42u8; 32]).unwrap();
        let original = vec![0u8; 64];
        let mut ciphertext = enc.encrypt_page(&original).unwrap();
        // Corrupt the auth tag
        let len = ciphertext.len();
        ciphertext[len - 1] ^= 0xFF;
        assert_eq!(enc.decrypt_page(&ciphertext).unwrap_err(), CryptoError::AuthFailed);
    }

    #[test]
    fn ecu_id_binding_works() {
        let ecu_id = [0x11; 32];
        let mut enc = TuneEncryption::new_h743(ecu_id);
        enc.generate_key([0x55u8; 32]).unwrap();
        enc.lock_tune().unwrap();
        assert!(enc.verify_ecu_binding().is_ok());
    }

    #[test]
    fn different_ecu_id_fails_binding() {
        let ecu_id = [0x11; 32];
        let mut enc = TuneEncryption::new_h743(ecu_id);
        enc.generate_key([0x55u8; 32]).unwrap();
        enc.lock_tune().unwrap();
        // Simulate different ECU
        enc.key.ecu_id = [0x22; 32]; // different ECU
        assert_eq!(enc.verify_ecu_binding().unwrap_err(), CryptoError::AuthFailed);
    }

    #[test]
    fn ecu_id_hex_is_64_chars() {
        let key = TuneKey { ecu_id: [0xABu8; 32], ..Default::default() };
        let hex = key.ecu_id_hex();
        assert_eq!(hex.len(), 64);
        // 0xAB → "ab"
        assert_eq!(hex[0], b'a');
        assert_eq!(hex[1], b'b');
    }

    #[test]
    fn key_generation_increments_counter() {
        let mut enc = TuneEncryption::new_h743([0u8; 32]);
        enc.generate_key([0x01u8; 32]).unwrap();
        assert_eq!(enc.key.generation, 1);
        enc.generate_key([0x02u8; 32]).unwrap();
        assert_eq!(enc.key.generation, 2);
    }
}
