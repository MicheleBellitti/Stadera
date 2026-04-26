//! `GET /me` — current authenticated user.
//!
//! Auth-protected: handler takes [`AuthUser`], so any unauthenticated
//! request short-circuits to `401 Unauthorized` in the extractor.

use axum::Json;
use axum::Router;
use axum::routing::get;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::ErrorBody;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/me", get(me))
}

#[derive(Serialize, ToSchema)]
pub(crate) struct MeResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

#[utoipa::path(
    get,
    path = "/me",
    tag = "user",
    responses(
        (status = 200, description = "Current authenticated user", body = MeResponse),
        (status = 401, description = "Missing or invalid session", body = ErrorBody),
    ),
    security(("session_cookie" = []))
)]
pub(crate) async fn me(user: AuthUser) -> Json<MeResponse> {
    Json(MeResponse {
        id: user.id,
        email: user.email,
        name: user.name,
    })
}
