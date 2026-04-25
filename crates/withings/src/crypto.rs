//! Token encryption (AES-256-GCM).
//!
//! Encryption layer for `access_token` and `refresh_token` stored in
//! `withings_credentials.access_token_enc` / `refresh_token_enc` (BYTEA).
//! Layout per encrypted blob: `nonce(12 bytes) || ciphertext || tag(16 bytes)`.
//! Key is loaded from `WITHINGS_TOKEN_KEY` (32 bytes hex) env var.
