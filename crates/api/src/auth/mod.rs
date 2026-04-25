//! Authentication: Google OAuth flow + cookie-based server-side sessions.
//!
//! Modules:
//! - [`cookies`]: cookie name constants and builder helpers.
//! - [`google`]: thin wrapper around the `oauth2` crate for Google OAuth.
//! - [`extractor`]: [`AuthUser`] request extractor — handlers that depend on
//!   it are auth-protected by construction.

pub mod cookies;
pub mod extractor;
pub mod google;

pub use extractor::AuthUser;
