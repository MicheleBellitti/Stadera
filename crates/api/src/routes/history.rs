//! `GET /history?from=&to=` — every measurement in `[from, to]`, ASC.
//!
//! `from` and `to` are RFC 3339 timestamps. Both required: this endpoint
//! is intended for tabular display so the caller decides the slice.

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use stadera_storage::StorageContext;

use crate::auth::AuthUser;
use crate::dto::MeasurementView;
use crate::error::AppError;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/history", get(history))
}

#[derive(Deserialize)]
struct HistoryQuery {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

#[derive(Serialize)]
struct HistoryResponse {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    measurements: Vec<MeasurementView>,
}

async fn history(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, AppError> {
    if params.from >= params.to {
        return Err(AppError::BadRequest("`from` must be before `to`".into()));
    }

    let storage = StorageContext::new(state.pool.clone());
    let measurements = storage
        .measurements()
        .list_for_user_between(user.id, params.from, params.to)
        .await?;

    let views: Vec<MeasurementView> = measurements.iter().map(MeasurementView::from).collect();

    Ok(Json(HistoryResponse {
        from: params.from,
        to: params.to,
        measurements: views,
    }))
}
