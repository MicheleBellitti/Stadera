use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use stadera_domain::{BodyFatPercent, LeanMass, Measurement, Source, Weight};
use stadera_storage::StorageContext;
use uuid::Uuid;

async fn create_test_user(ctx: &StorageContext) -> Uuid {
    ctx.users()
        .create("test@example.com", "Test")
        .await
        .unwrap()
}

fn sample_measurement(day: u32) -> Measurement {
    Measurement::new(
        Utc.with_ymd_and_hms(2026, 4, day, 8, 0, 0).unwrap(),
        Weight::new(80.0).unwrap(),
        Some(BodyFatPercent::new(20.0).unwrap()),
        Some(LeanMass::new(64.0).unwrap()),
        Source::Withings,
    )
}

#[sqlx::test]
async fn insert_and_get_by_id(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let m = sample_measurement(24);

    let id = ctx.measurements().insert(user_id, &m).await.unwrap();
    let got = ctx.measurements().get_by_id(id).await.unwrap().unwrap();

    assert_eq!(got.taken_at, m.taken_at);
    assert_eq!(got.weight.value(), m.weight.value());
    assert_eq!(
        got.body_fat.map(|bf| bf.value()),
        m.body_fat.map(|bf| bf.value())
    );
    assert_eq!(
        got.lean_mass.map(|lm| lm.value()),
        m.lean_mass.map(|lm| lm.value())
    );
    assert_eq!(got.source, m.source);
    Ok(())
}

#[sqlx::test]
async fn insert_without_optional_fields(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let m = Measurement::new(
        Utc.with_ymd_and_hms(2026, 4, 24, 8, 0, 0).unwrap(),
        Weight::new(75.0).unwrap(),
        None,
        None,
        Source::Manual,
    );

    let id = ctx.measurements().insert(user_id, &m).await.unwrap();
    let got = ctx.measurements().get_by_id(id).await.unwrap().unwrap();

    assert!(got.body_fat.is_none());
    assert!(got.lean_mass.is_none());
    assert_eq!(got.source, Source::Manual);
    Ok(())
}

#[sqlx::test]
async fn get_by_id_non_existent_returns_none(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let id = Uuid::now_v7();
    assert!(ctx.measurements().get_by_id(id).await.unwrap().is_none());
    Ok(())
}

#[sqlx::test]
async fn list_for_user_between_filters_window(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    for day in [20u32, 22, 25] {
        ctx.measurements()
            .insert(user_id, &sample_measurement(day))
            .await
            .unwrap();
    }

    let from = Utc.with_ymd_and_hms(2026, 4, 21, 0, 0, 0).unwrap();
    let to = Utc.with_ymd_and_hms(2026, 4, 24, 0, 0, 0).unwrap();
    let results = ctx
        .measurements()
        .list_for_user_between(user_id, from, to)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].taken_at.timestamp(),
        sample_measurement(22).taken_at.timestamp()
    );
    Ok(())
}

#[sqlx::test]
async fn list_for_user_between_ordered_asc(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    // insert in reverse order
    for day in [25u32, 22, 20] {
        ctx.measurements()
            .insert(user_id, &sample_measurement(day))
            .await
            .unwrap();
    }

    let from = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let to = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
    let results = ctx
        .measurements()
        .list_for_user_between(user_id, from, to)
        .await
        .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].taken_at < results[1].taken_at);
    assert!(results[1].taken_at < results[2].taken_at);
    Ok(())
}

#[sqlx::test]
async fn latest_for_user(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    for day in [20u32, 22, 25] {
        ctx.measurements()
            .insert(user_id, &sample_measurement(day))
            .await
            .unwrap();
    }

    let latest = ctx
        .measurements()
        .latest_for_user(user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.taken_at, sample_measurement(25).taken_at);
    Ok(())
}

#[sqlx::test]
async fn latest_for_user_no_measurements_returns_none(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    assert!(
        ctx.measurements()
            .latest_for_user(user_id)
            .await
            .unwrap()
            .is_none()
    );
    Ok(())
}

#[sqlx::test]
async fn list_for_user_latest_respects_limit_and_order(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    for day in 20u32..=24 {
        ctx.measurements()
            .insert(user_id, &sample_measurement(day))
            .await
            .unwrap();
    }

    let latest3 = ctx
        .measurements()
        .list_for_user_latest(user_id, 3)
        .await
        .unwrap();
    assert_eq!(latest3.len(), 3);
    // DESC order: first is the most recent
    assert!(latest3[0].taken_at > latest3[1].taken_at);
    assert!(latest3[1].taken_at > latest3[2].taken_at);
    assert_eq!(latest3[0].taken_at, sample_measurement(24).taken_at);
    Ok(())
}

#[sqlx::test]
async fn insert_batch(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    let measurements: Vec<Measurement> = (20u32..=22).map(sample_measurement).collect();

    let ids = ctx
        .measurements()
        .insert_batch(user_id, &measurements)
        .await
        .unwrap();
    assert_eq!(ids.len(), 3);

    let all = ctx
        .measurements()
        .list_for_user_latest(user_id, 10)
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
    Ok(())
}

#[sqlx::test]
async fn insert_batch_rolls_back_on_failure(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);

    // A user_id not present in `users` → FK violation on the first INSERT,
    // which should abort the transaction and leave no partial state.
    let fake_user_id = Uuid::now_v7();
    let measurements: Vec<Measurement> = (20u32..=22).map(sample_measurement).collect();

    let result = ctx
        .measurements()
        .insert_batch(fake_user_id, &measurements)
        .await;
    assert!(result.is_err(), "expected FK violation to fail the batch");

    // Create a real user afterward: no rows should have leaked into `measurements`.
    let real_user_id = create_test_user(&ctx).await;
    let all = ctx
        .measurements()
        .list_for_user_latest(real_user_id, 100)
        .await
        .unwrap();
    assert!(
        all.is_empty(),
        "rollback should have left the measurements table empty for any user"
    );
    Ok(())
}
