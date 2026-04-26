//! `GET /profile` and `PUT /profile` — read and upsert the user's metabolic profile.
//!
//! `PUT` is upsert: first call creates the row, subsequent calls overwrite
//! all fields (no PATCH semantics for now — the FE form is small enough).

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, put};
use chrono::NaiveDate;
use serde::Deserialize;
use stadera_domain::{Height, UserProfile, Weight};
use stadera_storage::StorageContext;

use crate::auth::AuthUser;
use crate::dto::{ProfileView, parse_activity_level, parse_sex};
use crate::error::AppError;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/profile", put(put_profile))
}

async fn get_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ProfileView>, AppError> {
    let storage = StorageContext::new(state.pool.clone());
    let profile = storage
        .user_profiles()
        .get_for_user(user.id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(ProfileView::from(&profile)))
}

#[derive(Deserialize)]
struct ProfilePayload {
    sex: String,
    birth_date: NaiveDate,
    height_cm: f64,
    activity_level: String,
    goal_weight_kg: f64,
}

async fn put_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<ProfilePayload>,
) -> Result<impl IntoResponse, AppError> {
    let sex = parse_sex(&payload.sex)
        .ok_or_else(|| AppError::BadRequest(format!("invalid sex: {}", payload.sex)))?;
    let activity = parse_activity_level(&payload.activity_level).ok_or_else(|| {
        AppError::BadRequest(format!(
            "invalid activity_level: {}",
            payload.activity_level,
        ))
    })?;
    let height = Height::new(payload.height_cm)
        .map_err(|e| AppError::BadRequest(format!("height_cm: {e}")))?;
    let goal_weight = Weight::new(payload.goal_weight_kg)
        .map_err(|e| AppError::BadRequest(format!("goal_weight_kg: {e}")))?;

    let profile = UserProfile {
        birth_date: payload.birth_date,
        sex,
        height,
        activity,
        goal_weight,
    };

    let storage = StorageContext::new(state.pool.clone());
    storage.user_profiles().upsert(user.id, &profile).await?;

    Ok((StatusCode::NO_CONTENT, ()))
}
