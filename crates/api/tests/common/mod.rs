//! Shared test helpers. Discovered automatically by Cargo as `mod common`
//! inside each integration test file. Not a test target itself.

#![allow(dead_code)] // helpers used by some test files but not all

use chrono::{Duration, Utc};
use sqlx::PgPool;
use stadera_api::config::{Config, GoogleConfig};
use stadera_storage::StorageContext;
use uuid::Uuid;

/// Build a `Config` suitable for tests: dummy Google credentials, dev cookie
/// settings, and an unused bind addr (`Router::oneshot` doesn't actually
/// listen).
pub fn test_config() -> Config {
    Config {
        database_url: String::new(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        frontend_origin: "http://localhost:3000".to_string(),
        cookie_secure: false,
        cookie_domain: None,
        google: GoogleConfig {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            redirect_url: "http://localhost:3000/auth/google/callback".to_string(),
        },
    }
}

/// Create a user + an active 1-day session and return both the user id
/// and a `Cookie:` header value ready to attach to test requests.
pub async fn login(pool: &PgPool, email: &str, name: &str) -> (Uuid, String) {
    let storage = StorageContext::new(pool.clone());
    let user_id = storage.users().create(email, name).await.unwrap();
    let session = storage
        .sessions()
        .create(user_id, Utc::now() + Duration::days(1))
        .await
        .unwrap();
    let cookie = format!("stadera_session={}", session.id);
    (user_id, cookie)
}
