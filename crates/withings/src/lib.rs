//! Stadera Withings integration: OAuth 2.0, token management, and Health Mate API client.
//!
//! Provides:
//! - [`oauth`]: OAuth2 authorization-code flow + refresh token handling
//! - [`crypto`]: AES-256-GCM encryption for tokens at rest
//! - [`client`]: Health Mate API HTTP client (measurements, etc.)
//! - [`types`]: wire types for Withings API responses
//! - [`error`]: error types for everything in this crate

pub mod client;
pub mod crypto;
pub mod error;
pub mod oauth;
pub mod types;

// Re-exports (e.g. `WithingsError`) added during the implementation step,
// once the types in the module skeletons exist.
