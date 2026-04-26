use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use http_body_util::BodyExt;
use sqlx::PgPool;
use stadera_domain::{Measurement, Source, Weight};
use stadera_storage::StorageContext;
use tower::ServiceExt;

mod common;

async fn seed_measurements(pool: &PgPool, user_id: uuid::Uuid) {
    let storage = StorageContext::new(pool.clone());
    for day in 1u32..=10 {
        storage
            .measurements()
            .insert(
                user_id,
                &Measurement::new(
                    Utc.with_ymd_and_hms(2026, 4, day, 8, 0, 0).unwrap(),
                    Weight::new(80.0 - (day as f64) * 0.1).unwrap(),
                    None,
                    None,
                    Source::Manual,
                ),
            )
            .await
            .unwrap();
    }
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn trend_returns_window(pool: PgPool) {
    let (user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;
    seed_measurements(&pool, user_id).await;

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    // Use a wide window (365 days) so all seeded measurements fall within it.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/trend?days=365")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let measurements = v["measurements"].as_array().unwrap();
    assert!(
        !measurements.is_empty(),
        "expected at least one measurement in the window"
    );
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn trend_rejects_invalid_days(pool: PgPool) {
    let (_user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/trend?days=0")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn history_filters_window(pool: PgPool) {
    let (user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;
    seed_measurements(&pool, user_id).await;

    let from = Utc.with_ymd_and_hms(2026, 4, 3, 0, 0, 0).unwrap();
    let to = Utc.with_ymd_and_hms(2026, 4, 7, 0, 0, 0).unwrap();

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let uri = format!(
        "/history?from={}&to={}",
        urlencoding::encode(&from.to_rfc3339()),
        urlencoding::encode(&to.to_rfc3339())
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let measurements = v["measurements"].as_array().unwrap();
    // Days 3, 4, 5, 6 fall in [Apr 3, Apr 7) — exclusive upper bound matches our SQL.
    assert_eq!(measurements.len(), 4);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn history_rejects_inverted_window(pool: PgPool) {
    let (_user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;

    let from = Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap();
    let to = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let uri = format!(
        "/history?from={}&to={}",
        urlencoding::encode(&from.to_rfc3339()),
        urlencoding::encode(&to.to_rfc3339()),
    );

    let resp = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
