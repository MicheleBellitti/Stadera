//! Health Mate API client.
//!
//! Endpoints used by Stadera:
//! - `/measure?action=getmeas` — list weight measurements for a user
//!
//! The client wraps `reqwest`, adds bearer-token auth from a `WithingsCredentials`,
//! and decodes the JSON envelope (`status`, `body`, `error`) used by Withings.
