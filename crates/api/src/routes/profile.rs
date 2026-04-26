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
use utoipa::ToSchema;

use crate::auth::AuthUser;
use crate::dto::{ProfileView, parse_activity_level, parse_sex};
use crate::error::{AppError, ErrorBody};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/profile", put(put_profile))
}

#[utoipa::path(
    get,
    path = "/profile",
    tag = "profile",
    responses(
        (status = 200, description = "Current profile", body = ProfileView),
        (status = 404, description = "Profile not yet set", body = ErrorBody),
        (status = 401, description = "Missing or invalid session", body = ErrorBody),
    ),
    security(("session_cookie" = []))
)]
pub(crate) async fn get_profile(
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

#[derive(Deserialize, ToSchema)]
pub(crate) struct ProfilePayload {
    /// One of: `male`, `female`.
    pub sex: String,
    pub birth_date: NaiveDate,
    pub height_cm: f64,
    /// One of: `sedentary`, `lightly_active`, `moderately_active`, `very_active`.
    pub activity_level: String,
    pub goal_weight_kg: f64,
}

#[utoipa::path(
    put,
    path = "/profile",
    tag = "profile",
    request_body = ProfilePayload,
    responses(
        (status = 204, description = "Profile saved"),
        (status = 400, description = "Invalid field", body = ErrorBody),
        (status = 401, description = "Missing or invalid session", body = ErrorBody),
    ),
    security(("session_cookie" = []))
)]
pub(crate) async fn put_profile(
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
