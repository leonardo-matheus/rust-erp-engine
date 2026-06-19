use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use crate::error::AppError;

pub struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
    pub fn new(key: &[u8]) -> Result<Self, AppError> {
        if key.len() != 32 {
            return Err(AppError::Encryption("Key must be 256 bits".into()));
        }
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| AppError::Encryption(format!("Cipher init: {}", e)))?;
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, AppError> {
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AppError::Encryption(format!("Encrypt: {}", e)))?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        Ok(B64.encode(&combined))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, AppError> {
        let combined = B64.decode(encoded)
            .map_err(|e| AppError::Encryption(format!("Base64 decode: {}", e)))?;
        if combined.len() < 12 {
            return Err(AppError::Encryption("Ciphertext too short".into()));
        }
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Encryption(format!("Decrypt: {}", e)))?;
        String::from_utf8(plaintext)
            .map_err(|e| AppError::Encryption(format!("UTF-8: {}", e)))
    }
}

/// Constant-time string comparison to prevent timing attacks
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    constant_time_eq::constant_time_eq(a, b)
}
