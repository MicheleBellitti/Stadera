use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{NaiveDate, TimeZone, Utc};
use http_body_util::BodyExt;
use sqlx::PgPool;
use stadera_domain::{
    ActivityLevel, BodyFatPercent, Height, LeanMass, Measurement, Sex, Source, UserProfile, Weight,
};
use stadera_storage::StorageContext;
use tower::ServiceExt;

mod common;

#[sqlx::test(migrations = "../storage/migrations")]
async fn today_unauthorized(pool: PgPool) {
    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/today")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn today_with_no_data_returns_skeleton(pool: PgPool) {
    let (_user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/today")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(v["user"]["email"], "alice@example.com");
    assert!(v["latest"].is_null());
    assert!(v["bmi"].is_null());
    assert!(v["daily_target"].is_null());
    // weekly_delta_kg may be null with no data — both null and 0 are acceptable shapes.
    assert!(v["weekly_delta_kg"].is_null() || v["weekly_delta_kg"].is_number());
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn today_with_data_includes_kpis(pool: PgPool) {
    let (user_id, cookie) = common::login(&pool, "alice@example.com", "Alice").await;
    let storage = StorageContext::new(pool.clone());

    storage
        .user_profiles()
        .upsert(
            user_id,
            &UserProfile {
                birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
                sex: Sex::Male,
                height: Height::new(175.0).unwrap(),
                activity: ActivityLevel::ModeratelyActive,
                goal_weight: Weight::new(75.0).unwrap(),
            },
        )
        .await
        .unwrap();

    storage
        .measurements()
        .insert(
            user_id,
            &Measurement::new(
                Utc.with_ymd_and_hms(2026, 4, 24, 8, 0, 0).unwrap(),
                Weight::new(80.0).unwrap(),
                Some(BodyFatPercent::new(20.0).unwrap()),
                Some(LeanMass::new(64.0).unwrap()),
                Source::Manual,
            ),
        )
        .await
        .unwrap();

    let state = stadera_api::AppState::new(pool, common::test_config());
    let app = stadera_api::router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/today")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(v["latest"]["weight_kg"], 80.0);
    assert_eq!(v["latest"]["body_fat_percent"], 20.0);
    assert_eq!(v["latest"]["lean_mass_kg"], 64.0);

    // BMI = 80 / (1.75^2) ≈ 26.122
    let bmi = v["bmi"].as_f64().unwrap();
    assert!((bmi - 26.122).abs() < 0.01, "got bmi {bmi}");

    // daily_target: BMR = 370 + 21.6 * 64 = 1752.4; TDEE = 1752.4 * 1.55 = 2716.22
    // kcal = 2716.22 - 500 = 2216.22; protein_g = 80 * 1.8 = 144
    let kcal = v["daily_target"]["kcal"].as_f64().unwrap();
    let protein = v["daily_target"]["protein_g"].as_f64().unwrap();
    assert!((kcal - 2216.22).abs() < 0.5, "got kcal {kcal}");
    assert!((protein - 144.0).abs() < 0.001, "got protein {protein}");
}
