use chrono::NaiveDate;
use sqlx::PgPool;
use stadera_domain::{ActivityLevel, Height, Sex, UserProfile, Weight};
use stadera_storage::StorageContext;
use uuid::Uuid;

fn sample_profile() -> UserProfile {
    UserProfile {
        birth_date: NaiveDate::from_ymd_opt(1990, 6, 15).unwrap(),
        sex: Sex::Male,
        height: Height::new(175.0).unwrap(),
        activity: ActivityLevel::ModeratelyActive,
        goal_weight: Weight::new(75.0).unwrap(),
    }
}

async fn create_test_user(ctx: &StorageContext) -> Uuid {
    ctx.users()
        .create("test@example.com", "Test")
        .await
        .unwrap()
}

#[sqlx::test]
async fn upsert_and_get_roundtrip(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let profile = sample_profile();

    ctx.user_profiles().upsert(user_id, &profile).await.unwrap();
    let got = ctx
        .user_profiles()
        .get_for_user(user_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(got.birth_date, profile.birth_date);
    assert_eq!(got.sex, profile.sex);
    assert_eq!(got.height.value(), profile.height.value());
    assert_eq!(got.activity, profile.activity);
    assert_eq!(got.goal_weight.value(), profile.goal_weight.value());
    Ok(())
}

#[sqlx::test]
async fn upsert_updates_existing(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    let mut profile = sample_profile();
    ctx.user_profiles().upsert(user_id, &profile).await.unwrap();

    profile.goal_weight = Weight::new(70.0).unwrap();
    profile.activity = ActivityLevel::VeryActive;
    ctx.user_profiles().upsert(user_id, &profile).await.unwrap();

    let got = ctx
        .user_profiles()
        .get_for_user(user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.goal_weight.value(), 70.0);
    assert_eq!(got.activity, ActivityLevel::VeryActive);
    Ok(())
}

#[sqlx::test]
async fn get_non_existent_returns_none(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let id = Uuid::now_v7();
    assert!(
        ctx.user_profiles()
            .get_for_user(id)
            .await
            .unwrap()
            .is_none()
    );
    Ok(())
}
