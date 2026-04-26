//! HTTP routes, grouped by resource.
//!
//! Each module exposes a `routes() -> Router<AppState>` constructor so the
//! top-level [`crate::router`] composes them via `Router::merge`.

pub mod auth;
pub mod health;
pub mod history;
pub mod me;
pub mod profile;
pub mod today;
pub mod trend;
