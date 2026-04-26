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
use stadera_domain::energy::{bmr_katch_mcardle, daily_target as compute_daily_target};
use stadera_domain::trend::compute_trend;
use stadera_storage::StorageContext;

use crate::auth::AuthUser;
use crate::dto::{DailyTargetView, MeasurementView, UserView};
use crate::error::AppError;
use crate::state::AppState;

/// Standard cutting deficit for a moderately active adult. Configurable
/// later via `user_profiles.deficit_kcal` if we ever expose it.
const KCAL_DEFICIT: f64 = 500.0;

/// Protein target in grams per kg of body weight. Conservative cut value.
const PROTEIN_PER_KG: f64 = 1.8;

pub fn routes() -> Router<AppState> {
    Router::new().route("/today", get(today))
}

#[derive(Serialize)]
struct TodayResponse {
    user: UserView,
    latest: Option<MeasurementView>,
    bmi: Option<f64>,
    weekly_delta_kg: Option<f64>,
    daily_target: Option<DailyTargetView>,
}

async fn today(
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

    // `daily_target` returns `Result` because the domain refuses to produce a
    // target below 1200 kcal (safe minimum). For the dashboard we surface
    // `null` in that case — the FE can render a "raise your goal" hint.
    let daily_target = match (latest.as_ref(), profile.as_ref()) {
        (Some(m), Some(p)) => m.lean_mass.and_then(|lm| {
            let bmr = bmr_katch_mcardle(lm);
            let tdee = bmr * p.activity.multiplier();
            compute_daily_target(tdee, m.weight, KCAL_DEFICIT, PROTEIN_PER_KG).ok()
        }),
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
