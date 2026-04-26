//! `GET /today` — at-a-glance KPIs for the dashboard home.
//!
//! Pulls the latest measurement, the user profile, and a 14-day window of
//! measurements. Computes BMI (if we have a height), weekly weight delta
//! (if we have at least two weeks of data), and a daily kcal/protein target
//! (if we have a recent lean-mass reading and a profile).
//!
//! Every section is optional — fresh users with no data get a sane response
//! with most fields `null`, instead of a 404.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use chrono::{Duration, Utc};
use serde::Serialize;
use stadera_domain::energy::{bmr_katch_mcardle, daily_target as compute_daily_target, tdee};
use stadera_domain::trend::compute_trend;
use stadera_storage::StorageContext;
use utoipa::ToSchema;

use crate::auth::AuthUser;
use crate::dto::{DailyTargetView, MeasurementView, UserView};
use crate::error::{AppError, ErrorBody};
use crate::state::AppState;

/// Standard cutting deficit for a moderately active adult. Configurable
/// later via `user_profiles.deficit_kcal` if we ever expose it.
const KCAL_DEFICIT: f64 = 500.0;

/// Protein target in grams per kg of body weight. Conservative cut value.
const PROTEIN_PER_KG: f64 = 1.8;

pub fn routes() -> Router<AppState> {
    Router::new().route("/today", get(today))
}

#[derive(Serialize, ToSchema)]
pub(crate) struct TodayResponse {
    pub user: UserView,
    pub latest: Option<MeasurementView>,
    pub bmi: Option<f64>,
    pub weekly_delta_kg: Option<f64>,
    pub daily_target: Option<DailyTargetView>,
}

#[utoipa::path(
    get,
    path = "/today",
    tag = "measurements",
    responses(
        (status = 200, description = "Today's KPIs", body = TodayResponse),
        (status = 401, description = "Missing or invalid session", body = ErrorBody),
    ),
    security(("session_cookie" = []))
)]
pub(crate) async fn today(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<TodayResponse>, AppError> {
    let storage = StorageContext::new(state.pool.clone());

    let latest = storage.measurements().latest_for_user(user.id).await?;
    let profile = storage.user_profiles().get_for_user(user.id).await?;

    let bmi = match (latest.as_ref(), profile.as_ref()) {
        (Some(m), Some(p)) => Some(m.bmi(p.height)),
        _ => None,
    };

    // Pull a 14-day window so `compute_trend` can produce a weekly delta.
    let now = Utc::now();
    let from = now - Duration::days(14);
    let window = storage
        .measurements()
        .list_for_user_between(user.id, from, now)
        .await?;
    let trend = compute_trend(&window);

    // For `daily_target` we want the most recent body-composition reading
    // (lean_mass) within the last 14 days, even if it isn't the absolute
    // latest measurement. A user who logs a weight-only manual entry today
    // shouldn't lose their KPI just because the Withings body-composition
    // reading from yesterday no longer wins on `taken_at`.
    //
    // `compute_daily_target` returns `Result` because the domain refuses to
    // produce a target below 1200 kcal (safe minimum). We surface `null` in
    // that case — the FE can render a "raise your goal" hint.
    let recent_lean_mass = window
        .iter()
        .rev() // window is ASC; reverse to scan newest-first within the bound
        .find_map(|m| m.lean_mass);

    let daily_target = match (latest.as_ref(), profile.as_ref(), recent_lean_mass) {
        (Some(m), Some(p), Some(lm)) => {
            let _bmr = bmr_katch_mcardle(lm);
            let tdee = tdee(lm, p.activity);
            compute_daily_target(tdee, m.weight, KCAL_DEFICIT, PROTEIN_PER_KG).ok()
        }
        _ => None,
    };

    Ok(Json(TodayResponse {
        user: UserView {
            id: user.id,
            email: user.email,
            name: user.name,
        },
        latest: latest.as_ref().map(MeasurementView::from),
        bmi,
        weekly_delta_kg: trend.weekly_delta_kg,
        daily_target: daily_target.map(DailyTargetView::from),
    }))
}
