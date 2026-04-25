//! Token encryption (AES-256-GCM).
//!
//! Encryption layer for `access_token` and `refresh_token` persisted in
//! `withings_credentials.{access_token_enc, refresh_token_enc}` (`BYTEA`).
//!
//! Layout of each encrypted blob:
//!
//! ```text
//! nonce (12 bytes) || ciphertext (variable) || authentication tag (16 bytes)
//! ```
//!
//! The 32-byte master key is loaded from the `WITHINGS_TOKEN_KEY` env var
//! (hex-encoded). Generate one with:
//!
//! ```sh
//! openssl rand -hex 32
//! ```
//!
//! In dev keep it under `.secrets/` (gitignored). In production the key lives
//! in GCP Secret Manager and is injected as an env var into Cloud Run.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;

// Re-export so downstream crates (e.g. `stadera-jobs`) can pass cipher
// references around without taking a direct dependency on `aes-gcm`.
pub use aes_gcm::Aes256Gcm as Cipher;

use crate::error::{WithingsError, WithingsResult};

/// Length of the AES-256 master key, in bytes.
pub const KEY_LEN: usize = 32;

/// Length of the AES-GCM nonce, in bytes.
const NONCE_LEN: usize = 12;

/// AES-GCM authentication tag length, in bytes.
const TAG_LEN: usize = 16;

/// Minimum size of an encrypted blob: nonce + zero-length ciphertext + tag.
const MIN_BLOB_LEN: usize = NONCE_LEN + TAG_LEN;

/// Build an [`Aes256Gcm`] cipher from a raw 32-byte key.
pub fn cipher_from_bytes(key: &[u8]) -> WithingsResult<Aes256Gcm> {
    if key.len() != KEY_LEN {
        return Err(WithingsError::Config(format!(
            "expected {KEY_LEN}-byte master key, got {} bytes",
            key.len()
        )));
    }
    Aes256Gcm::new_from_slice(key)
        .map_err(|e| WithingsError::Config(format!("invalid AES-256 key: {e}")))
}

/// Load the cipher from the `WITHINGS_TOKEN_KEY` env var
/// (must be a 64-character hex string of 32 bytes).
pub fn cipher_from_env() -> WithingsResult<Aes256Gcm> {
    let hex_str = std::env::var("WITHINGS_TOKEN_KEY")
        .map_err(|_| WithingsError::Config("WITHINGS_TOKEN_KEY env var is missing".into()))?;
    let key = hex::decode(hex_str.trim())
        .map_err(|e| WithingsError::Config(format!("WITHINGS_TOKEN_KEY is not valid hex: {e}")))?;
    cipher_from_bytes(&key)
}

/// Encrypt `plaintext`, returning `nonce || ciphertext || tag`.
///
/// A fresh random nonce is generated for every call (AES-GCM IS NOT SAFE
/// against nonce reuse with the same key — never reuse).
pub fn encrypt(cipher: &Aes256Gcm, plaintext: &[u8]) -> WithingsResult<Vec<u8>> {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| WithingsError::Encryption(e.to_string()))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt a `nonce || ciphertext || tag` blob.
///
/// Returns [`WithingsError::Decryption`] if the blob is malformed, the
/// authentication tag is invalid (tampering), or the key is wrong.
pub fn decrypt(cipher: &Aes256Gcm, blob: &[u8]) -> WithingsResult<Vec<u8>> {
    if blob.len() < MIN_BLOB_LEN {
        return Err(WithingsError::Decryption(format!(
            "ciphertext too short: {} bytes (minimum {MIN_BLOB_LEN})",
            blob.len()
        )));
    }
    let (nonce_bytes, rest) = blob.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, rest)
        .map_err(|e| WithingsError::Decryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cipher() -> Aes256Gcm {
        let key = [0x42u8; KEY_LEN];
        cipher_from_bytes(&key).unwrap()
    }

    #[test]
    fn roundtrip_recovers_plaintext() {
        let cipher = test_cipher();
        let plaintext = b"super-secret-access-token";
        let blob = encrypt(&cipher, plaintext).unwrap();
        let decrypted = decrypt(&cipher, &blob).unwrap();
        assert_eq!(plaintext.as_ref(), decrypted.as_slice());
    }

    #[test]
    fn each_encrypt_uses_a_fresh_nonce() {
        let cipher = test_cipher();
        let plaintext = b"same plaintext bytes";
        let blob1 = encrypt(&cipher, plaintext).unwrap();
        let blob2 = encrypt(&cipher, plaintext).unwrap();
        // AES-GCM with a fresh random nonce per call must produce
        // different ciphertext for the same plaintext.
        assert_ne!(blob1, blob2);
    }

    #[test]
    fn tampered_ciphertext_fails_authentication() {
        let cipher = test_cipher();
        let mut blob = encrypt(&cipher, b"payload").unwrap();
        // Flip a bit in the last byte (part of the auth tag).
        *blob.last_mut().unwrap() ^= 0x01;
        let result = decrypt(&cipher, &blob);
        assert!(matches!(result, Err(WithingsError::Decryption(_))));
    }

    #[test]
    fn truncated_blob_fails_with_clear_error() {
        let cipher = test_cipher();
        let short = vec![0u8; 10];
        let result = decrypt(&cipher, &short);
        assert!(matches!(result, Err(WithingsError::Decryption(_))));
    }

    #[test]
    fn wrong_key_fails_authentication() {
        let cipher_a = test_cipher();
        let cipher_b = cipher_from_bytes(&[0xFFu8; KEY_LEN]).unwrap();
        let blob = encrypt(&cipher_a, b"payload").unwrap();
        let result = decrypt(&cipher_b, &blob);
        assert!(matches!(result, Err(WithingsError::Decryption(_))));
    }

    #[test]
    fn cipher_from_bytes_rejects_wrong_length() {
        let too_short = vec![0u8; KEY_LEN - 1];
        assert!(matches!(
            cipher_from_bytes(&too_short),
            Err(WithingsError::Config(_))
        ));
    }
}
