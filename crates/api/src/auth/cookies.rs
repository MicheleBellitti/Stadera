//! Cookie name constants and helper builders.
//!
//! All cookies are `HttpOnly` and `SameSite=Lax`. The `Secure` flag is
//! set in production (HTTPS) via the `cookie_secure` config flag.
//!
//! `domain` is optional. When `None` (default) the browser scopes the
//! cookie to the exact host that set it — fine for dev (`localhost`)
//! and single-host prod. When `Some(".stadera.org")` the cookie is
//! shared across all subdomains, required when FE and BE are on
//! different subdomains of a common parent.
//!
//! Cleared cookies must use the **same domain** attribute as the
//! original — otherwise the browser sees the clear as a distinct
//! cookie and won't actually expire the live one.

use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

/// Long-lived session cookie carrying the session UUID (opaque to JS).
pub const SESSION_COOKIE: &str = "stadera_session";

/// Short-lived cookie holding the OAuth CSRF state for the round trip
/// from `/auth/google/start` → Google consent → `/auth/google/callback`.
pub const OAUTH_STATE_COOKIE: &str = "stadera_oauth_state";

/// Session cookie max age (30 days) — matches the row's `expires_at`.
const SESSION_TTL: Duration = Duration::days(30);

/// OAuth state cookie max age (10 minutes) — covers a normal consent flow.
const OAUTH_STATE_TTL: Duration = Duration::minutes(10);

pub fn build_session_cookie(value: String, secure: bool, domain: Option<&str>) -> Cookie<'static> {
    let mut c = Cookie::build((SESSION_COOKIE, value))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(SESSION_TTL)
        .build();
    if let Some(d) = domain {
        c.set_domain(d.to_string());
    }
    c
}

pub fn clear_session_cookie(secure: bool, domain: Option<&str>) -> Cookie<'static> {
    let mut c = Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::ZERO)
        .build();
    if let Some(d) = domain {
        c.set_domain(d.to_string());
    }
    c
}

pub fn build_oauth_state_cookie(
    value: String,
    secure: bool,
    domain: Option<&str>,
) -> Cookie<'static> {
    let mut c = Cookie::build((OAUTH_STATE_COOKIE, value))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/auth")
        .max_age(OAUTH_STATE_TTL)
        .build();
    if let Some(d) = domain {
        c.set_domain(d.to_string());
    }
    c
}

pub fn clear_oauth_state_cookie(secure: bool, domain: Option<&str>) -> Cookie<'static> {
    let mut c = Cookie::build((OAUTH_STATE_COOKIE, ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/auth")
        .max_age(Duration::ZERO)
        .build();
    if let Some(d) = domain {
        c.set_domain(d.to_string());
    }
    c
}
