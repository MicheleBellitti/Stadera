//! Authentication routes.
//!
//! - `GET  /auth/google/start`     — redirect the browser to Google's consent screen.
//! - `GET  /auth/google/callback`  — handle Google's redirect, create a session.
//! - `POST /auth/logout`           — delete the current session.

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};
use serde::Deserialize;
use stadera_storage::StorageContext;
use uuid::Uuid;

use crate::auth::cookies::{
    OAUTH_STATE_COOKIE, SESSION_COOKIE, build_oauth_state_cookie, build_session_cookie,
    clear_oauth_state_cookie, clear_session_cookie,
};
use crate::auth::google::GoogleClient;
use crate::error::AppError;
use crate::state::AppState;

/// Sessions live 30 days.
const SESSION_LIFETIME: Duration = Duration::days(30);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/google/start", get(start))
        .route("/auth/google/callback", get(callback))
        .route("/auth/logout", post(logout))
}

async fn start(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    let google = GoogleClient::new(state.config.google.clone())?;
    let (auth_url, csrf) = google.authorize_url()?;

    let jar = jar.add(build_oauth_state_cookie(
        csrf.secret().to_string(),
        state.config.cookie_secure,
        state.config.cookie_domain.as_deref(),
    ));

    Ok((jar, Redirect::to(auth_url.as_str())))
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

async fn callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(params): Query<CallbackQuery>,
) -> Result<(CookieJar, Redirect), AppError> {
    // 1. Validate the CSRF state we stashed in /auth/google/start.
    let expected_state = jar
        .get(OAUTH_STATE_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            AppError::BadRequest("missing oauth state cookie — expired or never started".into())
        })?;
    if expected_state != params.state {
        return Err(AppError::BadRequest("oauth state mismatch".into()));
    }

    // 2. Exchange the authorization code for an access token + profile.
    let google = GoogleClient::new(state.config.google.clone())?;
    let userinfo = google.exchange_code(params.code).await?;

    // 3. Find the user by email; create on first login.
    let storage = StorageContext::new(state.pool.clone());
    let user =
        match storage.users().get_by_email(&userinfo.email).await? {
            Some(u) => u,
            None => {
                let id = storage
                    .users()
                    .create(&userinfo.email, &userinfo.name)
                    .await?;
                storage.users().get_by_id(id).await?.ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("just-created user vanished"))
                })?
            }
        };

    // 4. Create a server-side session.
    let session = storage
        .sessions()
        .create(user.id, Utc::now() + SESSION_LIFETIME)
        .await?;

    // 5. Swap cookies: drop the oauth state cookie, set the session cookie.
    let cookie_domain = state.config.cookie_domain.as_deref();
    let jar = jar
        .add(clear_oauth_state_cookie(
            state.config.cookie_secure,
            cookie_domain,
        ))
        .add(build_session_cookie(
            session.id.to_string(),
            state.config.cookie_secure,
            cookie_domain,
        ));

    // 6. Hand the user back to the frontend.
    Ok((jar, Redirect::to(&state.config.frontend_origin)))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE)
        && let Ok(session_id) = Uuid::parse_str(cookie.value())
    {
        let storage = StorageContext::new(state.pool.clone());
        // Best-effort: ignore failures (cookie might be stale).
        let _ = storage.sessions().delete(session_id).await;
    }

    let jar = jar.add(clear_session_cookie(
        state.config.cookie_secure,
        state.config.cookie_domain.as_deref(),
    ));
    Ok((jar, Redirect::to(&state.config.frontend_origin)))
}
