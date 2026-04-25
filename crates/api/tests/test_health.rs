use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test]
async fn health_returns_200_ok_json(pool: PgPool) {
    let state = stadera_api::AppState::new(pool);
    let app = stadera_api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body, serde_json::json!({ "status": "ok" }));
}

#[sqlx::test]
async fn unknown_route_returns_404(pool: PgPool) {
    let state = stadera_api::AppState::new(pool);
    let app = stadera_api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/this-does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
