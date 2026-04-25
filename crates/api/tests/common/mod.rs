//! Shared test helpers. Discovered automatically by Cargo as `mod common`
//! inside each integration test file. Not a test target itself.

use stadera_api::config::{Config, GoogleConfig};

/// Build a `Config` suitable for tests: dummy Google credentials, dev cookie
/// settings, and an unused bind addr (`Router::oneshot` doesn't actually
/// listen).
pub fn test_config() -> Config {
    Config {
        database_url: String::new(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        frontend_origin: "http://localhost:3000".to_string(),
        cookie_secure: false,
        google: GoogleConfig {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            redirect_url: "http://localhost:3000/auth/google/callback".to_string(),
        },
    }
}
