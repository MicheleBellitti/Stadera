use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

mod common;

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_profile_returns_404_when_not_set(pool: PgPool) {
    let (_user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/profile")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn put_profile_then_get_roundtrips(pool: PgPool) {
    let (_user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state.clone());

    let body = serde_json::json!({
        "sex": "male",
        "birth_date": "1990-06-15",
        "height_cm": 175.0,
        "activity_level": "moderately_active",
        "goal_weight_kg": 75.0
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/profile")
                .header("cookie", cookie.clone())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Read it back via GET /profile.
    let app = stadera_api::router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/profile")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["sex"], "male");
    assert_eq!(v["birth_date"], "1990-06-15");
    assert_eq!(v["height_cm"], 175.0);
    assert_eq!(v["activity_level"], "moderately_active");
    assert_eq!(v["goal_weight_kg"], 75.0);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn put_profile_rejects_invalid_sex(pool: PgPool) {
    let (_user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let body = serde_json::json!({
        "sex": "other",
        "birth_date": "1990-06-15",
        "height_cm": 175.0,
        "activity_level": "moderately_active",
        "goal_weight_kg": 75.0
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/profile")
                .header("cookie", cookie)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn put_profile_rejects_out_of_range_height(pool: PgPool) {
    let (_user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let body = serde_json::json!({
        "sex": "female",
        "birth_date": "1990-06-15",
        "height_cm": 1000.0, // out of domain range
        "activity_level": "sedentary",
        "goal_weight_kg": 60.0
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/profile")
                .header("cookie", cookie)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
