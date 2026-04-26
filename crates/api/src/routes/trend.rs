//! `GET /trend?days=N` — measurements over a recent window plus aggregate stats.
//!
//! Default window: 30 days. Maximum allowed: 365.

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::routing::get;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use stadera_domain::trend::compute_trend;
use stadera_storage::StorageContext;

use crate::auth::AuthUser;
use crate::dto::MeasurementView;
use crate::error::AppError;
use crate::state::AppState;

const DEFAULT_DAYS: i64 = 30;
const MAX_DAYS: i64 = 365;

pub fn routes() -> Router<AppState> {
    Router::new().route("/trend", get(trend))
}

#[derive(Deserialize)]
struct TrendQuery {
    days: Option<i64>,
}

#[derive(Serialize)]
struct TrendResponse {
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
    measurements: Vec<MeasurementView>,
    moving_average_7d_kg: Option<f64>,
    weekly_delta_kg: Option<f64>,
}

async fn trend(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<TrendQuery>,
) -> Result<Json<TrendResponse>, AppError> {
    let days = params.days.unwrap_or(DEFAULT_DAYS);
    if !(1..=MAX_DAYS).contains(&days) {
        return Err(AppError::BadRequest(format!(
            "`days` must be between 1 and {MAX_DAYS}",
        )));
    }

    let to = Utc::now();
    let from = to - Duration::days(days);

    let storage = StorageContext::new(state.pool.clone());
    let measurements = storage
        .measurements()
        .list_for_user_between(user.id, from, to)
        .await?;

    let stats = compute_trend(&measurements);
    let views: Vec<MeasurementView> = measurements.iter().map(MeasurementView::from).collect();

    Ok(Json(TrendResponse {
        from,
        to,
        measurements: views,
        moving_average_7d_kg: stats.moving_average_7d.map(|w| w.value()),
        weekly_delta_kg: stats.weekly_delta_kg,
    }))
}
