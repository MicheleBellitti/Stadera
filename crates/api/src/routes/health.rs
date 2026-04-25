//! Liveness probe.
//!
//! Returns `200 OK { "status": "ok" }` if the process is alive. **Does not
//! ping the database** — that's a readiness concern, addressed via a
//! separate `/ready` endpoint in M7 when the deploy story stabilizes.
//! Liveness probes are fired at high frequency and must be cheap.

use axum::Json;
use axum::Router;
use axum::routing::get;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
