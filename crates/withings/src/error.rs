//! Error types for the Withings integration.
//!
//! `WithingsError` is the unified error returned by every fallible operation in
//! this crate: HTTP calls, OAuth flow, token encryption, and response parsing.

use thiserror::Error;

pub type WithingsResult<T> = Result<T, WithingsError>;

#[derive(Debug, Error)]
pub enum WithingsError {
    /// Network or HTTP-level failure (DNS, TLS, timeout, decoding the body, …).
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Withings returned `status != 0` in the response envelope.
    /// See <https://developer.withings.com/api-reference#tag/return-status>.
    #[error("Withings API error (status {status}): {message}")]
    Api { status: i64, message: String },

    /// OAuth-specific failure (invalid grant, expired refresh token, malformed
    /// authorization response, …). Wraps a reason string from `oauth2` or our flow.
    #[error("OAuth error: {0}")]
    Oauth(String),

    /// Authentication problem distinct from Oauth: e.g. no credentials stored
    /// for the user, or stored access token expired and refresh failed.
    #[error("authentication error: {0}")]
    Auth(String),

    /// Withings has temporarily rate-limited us. Caller should back off.
    #[error("rate limited by Withings (retry after: {retry_after_secs:?}s)")]
    RateLimit { retry_after_secs: Option<u64> },

    /// Failed to decrypt a token blob (wrong key, tampered ciphertext, etc.).
    #[error("token decryption failed: {0}")]
    Decryption(String),

    /// Failed to encrypt a token blob (e.g. RNG failure).
    #[error("token encryption failed: {0}")]
    Encryption(String),

    /// Configuration is missing or invalid (e.g. `WITHINGS_TOKEN_KEY` env var).
    #[error("invalid configuration: {0}")]
    Config(String),

    /// Withings returned a successful envelope but the body did not match the
    /// expected shape (missing field, wrong type, …).
    #[error("unexpected response shape: {0}")]
    UnexpectedResponse(String),
}
