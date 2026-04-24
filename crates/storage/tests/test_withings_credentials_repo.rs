use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use stadera_storage::{StorageContext, WithingsCredentials};
use uuid::Uuid;

async fn create_test_user(ctx: &StorageContext) -> Uuid {
    ctx.users()
        .create("test@example.com", "Test")
        .await
        .unwrap()
}

fn sample_credentials(user_id: Uuid) -> WithingsCredentials {
    WithingsCredentials {
        user_id,
        access_token_enc: vec![1, 2, 3, 4],
        refresh_token_enc: vec![5, 6, 7, 8],
        expires_at: Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap(),
        scope: "user.metrics".to_string(),
    }
}

#[sqlx::test]
async fn upsert_and_get_roundtrip(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let creds = sample_credentials(user_id);

    ctx.withings_credentials().upsert(&creds).await.unwrap();
    let got = ctx
        .withings_credentials()
        .get(user_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(got.user_id, creds.user_id);
    assert_eq!(got.access_token_enc, creds.access_token_enc);
    assert_eq!(got.refresh_token_enc, creds.refresh_token_enc);
    assert_eq!(got.expires_at, creds.expires_at);
    assert_eq!(got.scope, creds.scope);
    Ok(())
}

#[sqlx::test]
async fn upsert_updates_existing(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;

    let mut creds = sample_credentials(user_id);
    ctx.withings_credentials().upsert(&creds).await.unwrap();

    creds.access_token_enc = vec![9, 9, 9];
    creds.scope = "user.metrics,user.info".to_string();
    ctx.withings_credentials().upsert(&creds).await.unwrap();

    let got = ctx
        .withings_credentials()
        .get(user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.access_token_enc, vec![9, 9, 9]);
    assert_eq!(got.scope, "user.metrics,user.info");
    Ok(())
}

#[sqlx::test]
async fn delete_removes_row(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let creds = sample_credentials(user_id);

    ctx.withings_credentials().upsert(&creds).await.unwrap();
    ctx.withings_credentials().delete(user_id).await.unwrap();

    let got = ctx.withings_credentials().get(user_id).await.unwrap();
    assert!(got.is_none());
    Ok(())
}

#[sqlx::test]
async fn get_non_existent_returns_none(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let id = Uuid::now_v7();
    assert!(ctx.withings_credentials().get(id).await.unwrap().is_none());
    Ok(())
}

#[sqlx::test]
async fn delete_non_existent_is_noop(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let id = Uuid::now_v7();
    // Deleting something that doesn't exist should not error
    ctx.withings_credentials().delete(id).await.unwrap();
    Ok(())
}
