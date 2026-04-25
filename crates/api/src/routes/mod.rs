//! HTTP routes, grouped by resource.
//!
//! Each module exposes a `routes() -> Router<AppState>` constructor so the
//! top-level [`crate::router`] composes them via `Router::merge`.
//!
//! Route inventory (built out in subsequent M5 PRs):
//! - [`health`]: liveness probe.
//! - (M5 PR B) `auth`: Google OAuth sign-in + session cookie issuance.
//! - (M5 PR C) `me`, `today`, `trend`, `history`, `profile`: domain endpoints.

pub mod health;
