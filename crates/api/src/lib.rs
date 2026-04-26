//! Stadera HTTP API (axum).
//!
//! The crate is split into a library (this file + modules) and a binary
//! (`src/main.rs`) so the router can be exercised in tests via
//! `Router::oneshot` without binding a TCP listener.
//!
//! Module map:
//! - [`config`]: typed config loaded from environment variables.
//! - [`state`]: [`AppState`] shared across handlers.
//! - [`error`]: [`AppError`] + `IntoResponse` impl. Internal failures
//!   are logged but never leaked to clients.
//! - [`auth`]: Google OAuth flow, cookies, [`auth::AuthUser`] extractor.
//! - [`routes`]: per-resource route trees, composed in [`router`].

pub mod auth;
pub mod config;
pub mod dto;
pub mod error;
pub mod openapi;
pub mod routes;
pub mod state;

pub use error::AppError;
pub use state::AppState;

use std::time::Duration;

use axum::Router;
use axum::http::{HeaderName, HeaderValue, Method, StatusCode, header};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::openapi::ApiDoc;

/// Build the top-level router with all layers wired up.
pub fn router(state: AppState) -> Router {
    let cors = build_cors(&state.config.frontend_origin);

    Router::new()
        .merge(routes::health::routes())
        .merge(routes::auth::routes())
        .merge(routes::me::routes())
        .merge(routes::today::routes())
        .merge(routes::trend::routes())
        .merge(routes::history::routes())
        .merge(routes::profile::routes())
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

/// CORS layer scoped to the configured frontend origin, with credentials
/// (cookies) enabled. Browsers will only accept `Access-Control-Allow-Origin`
/// echoing a specific origin (not `*`) when credentials are involved.
fn build_cors(frontend_origin: &str) -> CorsLayer {
    let allowed_origin: HeaderValue = frontend_origin
        .parse()
        .expect("FRONTEND_ORIGIN must be a valid header value");
    let allowed_headers: [HeaderName; 2] = [header::CONTENT_TYPE, header::ACCEPT];
    CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(allowed_headers)
}
