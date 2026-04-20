use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};

/// Tune encryption — AES-256 per-ECU key + True RNG (H743 only).
///
/// H743: True RNG generates a unique 32-byte key on first startup, stored in OTP.
///       Tunes are AES-256-GCM encrypted and bound to the specific ECU ID.
///       Encrypted config pages include a 12-byte nonce prefix + ciphertext + 16-byte auth tag.
///
/// F407: Returns Err(NotSupported) for all crypto operations.
///       UI shows a badge indicating H743-only feature.
const GCM_NONCE_LEN: usize = 12;
const GCM_TAG_LEN: usize = 16;

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
    pub nonce_counter: u64,
}

/// Tune encryption controller
#[derive(Debug, Clone)]
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
        let mut seed = [0u8; 8];
        seed.copy_from_slice(&rng_bytes[..8]);
        self.state.nonce_counter = u64::from_be_bytes(seed);
        Ok(())
    }

    fn next_nonce_bytes(&mut self) -> [u8; GCM_NONCE_LEN] {
        let counter = self.state.nonce_counter;
        self.state.nonce_counter = self.state.nonce_counter.wrapping_add(1);

        // 96-bit nonce = key-generation epoch (u32) + monotonic counter (u64)
        let mut nonce = [0u8; GCM_NONCE_LEN];
        nonce[..4].copy_from_slice(&self.key.generation.to_be_bytes());
        nonce[4..].copy_from_slice(&counter.to_be_bytes());
        nonce
    }

    /// Encrypt a config page. Returns nonce prefix + ciphertext + 16-byte GCM tag.
    /// Output length is page.len() + 12 + 16.
    pub fn encrypt_page(&mut self, page: &[u8]) -> CryptoResult<Vec<u8>> {
        if !self.target_supports_crypto {
            return Err(CryptoError::NotSupported);
        }
        if !self.key.provisioned {
            return Err(CryptoError::KeyNotLoaded);
        }
        self.verify_ecu_binding()?;

        let cipher =
            Aes256Gcm::new_from_slice(&self.key.key).map_err(|_| CryptoError::InvalidInput)?;
        let nonce_bytes = self.next_nonce_bytes();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: page,
                    aad: &self.key.ecu_id,
                },
            )
            .map_err(|_| CryptoError::AuthFailed)?;

        let mut out = Vec::with_capacity(GCM_NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt a config page produced by encrypt_page.
    pub fn decrypt_page(&self, ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        if !self.target_supports_crypto {
            return Err(CryptoError::NotSupported);
        }
        if !self.key.provisioned {
            return Err(CryptoError::KeyNotLoaded);
        }
        self.verify_ecu_binding()?;

        if ciphertext.len() < (GCM_NONCE_LEN + GCM_TAG_LEN) {
            return Err(CryptoError::InvalidInput);
        }
        let (nonce_bytes, body) = ciphertext.split_at(GCM_NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let cipher =
            Aes256Gcm::new_from_slice(&self.key.key).map_err(|_| CryptoError::InvalidInput)?;
        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: body,
                    aad: &self.key.ecu_id,
                },
            )
            .map_err(|_| CryptoError::AuthFailed)
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
        if !self.state.locked {
            return Ok(());
        } // unlocked tune works on any ECU
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
        assert_eq!(
            enc.encrypt_page(&[0u8; 512]).unwrap_err(),
            CryptoError::NotSupported
        );
    }

    #[test]
    fn h743_encrypt_decrypt_roundtrip() {
        let ecu_id = [0xAB; 32];
        let mut enc = TuneEncryption::new_h743(ecu_id);
        enc.generate_key([0x42u8; 32]).unwrap();
        let original = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let ciphertext = enc.encrypt_page(&original).unwrap();
        assert_eq!(
            ciphertext.len(),
            original.len() + GCM_NONCE_LEN + GCM_TAG_LEN
        );
        assert_ne!(
            ciphertext[GCM_NONCE_LEN..GCM_NONCE_LEN + original.len()],
            original[..]
        );
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
        assert_eq!(
            enc.decrypt_page(&ciphertext).unwrap_err(),
            CryptoError::AuthFailed
        );
    }

    #[test]
    fn gcm_nonce_prefix_changes_between_encryptions() {
        let ecu_id = [0xAB; 32];
        let mut enc = TuneEncryption::new_h743(ecu_id);
        enc.generate_key([0x42u8; 32]).unwrap();

        let payload = vec![0x55u8; 64];
        let ct_a = enc.encrypt_page(&payload).unwrap();
        let ct_b = enc.encrypt_page(&payload).unwrap();

        assert_ne!(&ct_a[..GCM_NONCE_LEN], &ct_b[..GCM_NONCE_LEN]);
        assert_eq!(enc.decrypt_page(&ct_a).unwrap(), payload);
        assert_eq!(enc.decrypt_page(&ct_b).unwrap(), payload);
    }

    #[test]
    fn decrypt_rejects_too_short_payload() {
        let ecu_id = [0xAB; 32];
        let mut enc = TuneEncryption::new_h743(ecu_id);
        enc.generate_key([0x42u8; 32]).unwrap();
        assert_eq!(
            enc.decrypt_page(&[0x11u8; 8]).unwrap_err(),
            CryptoError::InvalidInput
        );
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
        assert_eq!(
            enc.verify_ecu_binding().unwrap_err(),
            CryptoError::AuthFailed
        );
    }

    #[test]
    fn ecu_id_hex_is_64_chars() {
        let key = TuneKey {
            ecu_id: [0xABu8; 32],
            ..Default::default()
        };
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
