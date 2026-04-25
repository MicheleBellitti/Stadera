//! Stadera HTTP API (axum).
//!
//! The crate is split into a library (this file + modules) and a binary
//! (`src/main.rs`) so the router can be exercised in tests via
//! `Router::oneshot` without binding a TCP listener.
//!
//! Module map:
//! - [`config`]: typed config loaded from environment variables.
//! - [`state`]: [`AppState`] shared across handlers (`Clone` cheap, internally `Arc`-backed).
//! - [`error`]: [`AppError`] + `IntoResponse` impl. Internal failures
//!   are logged but never leaked to clients.
//! - [`routes`]: per-resource route trees, composed in [`router`].

pub mod config;
pub mod error;
pub mod routes;
pub mod state;

pub use error::AppError;
pub use state::AppState;

use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

/// Build the top-level router with all layers wired up.
///
/// Tightening of [`CorsLayer::permissive`] to a strict allow-list happens
/// in M5 PR B once the frontend deploy URL is known.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(routes::health::routes())
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
