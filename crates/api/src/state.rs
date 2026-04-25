//! Shared application state.
//!
//! `AppState` is `Clone`-cheap: it wraps a `PgPool` (sqlx internally `Arc`s it)
//! and a `Config` (small `String`s + addr). Handlers extract it via
//! [`axum::extract::State`].

use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }
}
