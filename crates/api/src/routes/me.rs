//! `GET /me` — current authenticated user.
//!
//! Auth-protected: handler takes [`AuthUser`], so any unauthenticated
//! request short-circuits to `401 Unauthorized` in the extractor.

use axum::Json;
use axum::Router;
use axum::routing::get;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/me", get(me))
}

#[derive(Serialize)]
struct MeResponse {
    id: Uuid,
    email: String,
    name: String,
}

async fn me(user: AuthUser) -> Json<MeResponse> {
    Json(MeResponse {
        id: user.id,
        email: user.email,
        name: user.name,
    })
}
