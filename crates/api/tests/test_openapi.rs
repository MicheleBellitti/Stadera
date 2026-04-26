use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

mod common;

#[sqlx::test(migrations = "../storage/migrations")]
async fn openapi_json_is_valid_spec(pool: PgPool) {
    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let spec: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Top-level OpenAPI fields.
    assert!(
        spec["openapi"].as_str().unwrap().starts_with("3."),
        "expected OpenAPI 3.x, got {:?}",
        spec["openapi"]
    );
    assert_eq!(spec["info"]["title"], "Stadera API");
    assert_eq!(spec["info"]["version"], "0.1.0");

    // All annotated endpoints are present.
    let paths = spec["paths"].as_object().expect("paths is an object");
    for required in ["/health", "/me", "/today", "/trend", "/history", "/profile"] {
        assert!(
            paths.contains_key(required),
            "missing path {required} in OpenAPI spec",
        );
    }

    // /profile must document both GET and PUT.
    let profile_ops = spec["paths"]["/profile"].as_object().unwrap();
    assert!(profile_ops.contains_key("get"));
    assert!(profile_ops.contains_key("put"));

    // Security scheme `session_cookie` is registered.
    assert_eq!(
        spec["components"]["securitySchemes"]["session_cookie"]["in"],
        "cookie"
    );
    assert_eq!(
        spec["components"]["securitySchemes"]["session_cookie"]["name"],
        "stadera_session"
    );

    // Schemas are emitted.
    let schemas = spec["components"]["schemas"].as_object().unwrap();
    for required in [
        "UserView",
        "MeasurementView",
        "DailyTargetView",
        "ProfileView",
        "TodayResponse",
        "TrendResponse",
        "HistoryResponse",
        "ProfilePayload",
        "ErrorBody",
    ] {
        assert!(
            schemas.contains_key(required),
            "missing schema {required} in OpenAPI components",
        );
    }
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn swagger_ui_is_served(pool: PgPool) {
    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    // SwaggerUi typically redirects `/docs` -> `/docs/` and serves the
    // index. Either an OK or a redirect is acceptable behaviour.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/docs/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status() == StatusCode::OK || resp.status().is_redirection(),
        "expected 200 or 3xx for /docs/, got {}",
        resp.status()
    );
}
