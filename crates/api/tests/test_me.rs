use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use sqlx::PgPool;
use stadera_storage::StorageContext;
use tower::ServiceExt;

mod common;

#[sqlx::test(migrations = "../storage/migrations")]
async fn me_without_cookie_is_unauthorized(pool: PgPool) {
    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let response = app
        .oneshot(Request::builder().uri("/me").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn me_with_invalid_cookie_is_unauthorized(pool: PgPool) {
    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/me")
                .header("cookie", "stadera_session=not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn me_with_unknown_session_is_unauthorized(pool: PgPool) {
    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    // Well-formed UUID but no matching session row.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/me")
                .header(
                    "cookie",
                    format!("stadera_session={}", uuid::Uuid::now_v7()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn me_with_valid_session_returns_user(pool: PgPool) {
    let storage = StorageContext::new(pool.clone());
    let user_id = storage
        .users()
        .create("alice@example.com", "Alice")
        .await
        .unwrap();
    let session = storage
        .sessions()
        .create(user_id, Utc::now() + Duration::days(1))
        .await
        .unwrap();

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/me")
                .header("cookie", format!("stadera_session={}", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["id"], user_id.to_string());
    assert_eq!(body["email"], "alice@example.com");
    assert_eq!(body["name"], "Alice");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn me_with_expired_session_is_unauthorized(pool: PgPool) {
    let storage = StorageContext::new(pool.clone());
    let user_id = storage
        .users()
        .create("expired@example.com", "Expired")
        .await
        .unwrap();
    // Session expired 10 minutes ago.
    let session = storage
        .sessions()
        .create(user_id, Utc::now() - Duration::minutes(10))
        .await
        .unwrap();

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/me")
                .header("cookie", format!("stadera_session={}", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
