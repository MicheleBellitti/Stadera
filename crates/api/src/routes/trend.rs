//! `GET /trend?days=N` — measurements over a recent window plus aggregate stats.
//!
//! Default window: 30 days. Maximum allowed: 365.

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::routing::get;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use stadera_domain::trend::compute_trend;
use stadera_storage::StorageContext;
use utoipa::{IntoParams, ToSchema};

use crate::auth::AuthUser;
use crate::dto::MeasurementView;
use crate::error::{AppError, ErrorBody};
use crate::state::AppState;

const DEFAULT_DAYS: i64 = 30;
const MAX_DAYS: i64 = 365;

pub fn routes() -> Router<AppState> {
    Router::new().route("/trend", get(trend))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct TrendQuery {
    /// Window length in days (default 30, max 365).
    pub days: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct TrendResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub measurements: Vec<MeasurementView>,
    pub moving_average_7d_kg: Option<f64>,
    pub weekly_delta_kg: Option<f64>,
}

#[utoipa::path(
    get,
    path = "/trend",
    tag = "measurements",
    params(TrendQuery),
    responses(
        (status = 200, description = "Measurements + aggregate stats over the window", body = TrendResponse),
        (status = 400, description = "`days` out of range", body = ErrorBody),
        (status = 401, description = "Missing or invalid session", body = ErrorBody),
    ),
    security(("session_cookie" = []))
)]
pub(crate) async fn trend(
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
