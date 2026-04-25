//! Shared application state.
//!
//! `AppState` is `Clone`-cheap: it wraps a `PgPool` (which is internally
//! `Arc`-backed by sqlx). Handlers extract it via [`axum::extract::State`].

use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
