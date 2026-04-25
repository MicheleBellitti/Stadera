//! `AuthUser` request extractor.
//!
//! Handlers that take an `AuthUser` parameter are auth-protected by
//! construction: if the request is missing or has an invalid session
//! cookie, the extractor returns `AppError::Unauthorized` and the
//! handler is never invoked.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use stadera_storage::StorageContext;
use uuid::Uuid;

use crate::auth::cookies::SESSION_COOKIE;
use crate::error::AppError;
use crate::state::AppState;

/// Authenticated user injected into protected handlers.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        let session_id_str = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(AppError::Unauthorized)?;
        let session_id = Uuid::parse_str(&session_id_str).map_err(|_| AppError::Unauthorized)?;

        let storage = StorageContext::new(state.pool.clone());

        let session = storage
            .sessions()
            .get_active(session_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        // Defense-in-depth: get_active filters expired in SQL, but check again
        // here in case clock skew between Postgres and the app makes the SQL
        // check off by a second.
        if session.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }

        // Best-effort touch so we have an idle timeout signal in the future.
        let _ = storage.sessions().touch(session_id).await;

        let user = storage
            .users()
            .get_by_id(session.user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        Ok(AuthUser {
            id: user.id,
            email: user.email,
            name: user.name,
        })
    }
}
