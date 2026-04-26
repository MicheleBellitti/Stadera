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
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub(crate) struct HealthResponse {
    pub status: &'static str,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Process is alive", body = HealthResponse)
    )
)]
pub(crate) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
