use chrono::{Duration, Utc};
use sqlx::PgPool;
use stadera_storage::StorageContext;
use uuid::Uuid;

async fn create_test_user(ctx: &StorageContext) -> Uuid {
    ctx.users()
        .create("test@example.com", "Test")
        .await
        .unwrap()
}

#[sqlx::test]
async fn create_and_get_active(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let expires = Utc::now() + Duration::days(30);

    let created = ctx.sessions().create(user_id, expires).await.unwrap();
    assert_eq!(created.user_id, user_id);

    let got = ctx
        .sessions()
        .get_active(created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.id, created.id);
    assert_eq!(got.user_id, user_id);
    Ok(())
}

#[sqlx::test]
async fn get_active_excludes_expired(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    // Already expired (10 minutes ago).
    let expires = Utc::now() - Duration::minutes(10);

    let created = ctx.sessions().create(user_id, expires).await.unwrap();
    let got = ctx.sessions().get_active(created.id).await.unwrap();
    assert!(
        got.is_none(),
        "expired sessions are not returned by get_active"
    );
    Ok(())
}

#[sqlx::test]
async fn get_active_non_existent(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let id = Uuid::now_v7();
    assert!(ctx.sessions().get_active(id).await.unwrap().is_none());
    Ok(())
}

#[sqlx::test]
async fn touch_updates_last_seen_at(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let created = ctx
        .sessions()
        .create(user_id, Utc::now() + Duration::days(1))
        .await
        .unwrap();
    let original_last_seen = created.last_seen_at;

    // Sleep a bit to ensure the timestamp changes.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    ctx.sessions().touch(created.id).await.unwrap();

    let after = ctx
        .sessions()
        .get_active(created.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        after.last_seen_at > original_last_seen,
        "touch should advance last_seen_at",
    );
    Ok(())
}

#[sqlx::test]
async fn delete_removes_session(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let user_id = create_test_user(&ctx).await;
    let created = ctx
        .sessions()
        .create(user_id, Utc::now() + Duration::days(1))
        .await
        .unwrap();

    ctx.sessions().delete(created.id).await.unwrap();
    assert!(
        ctx.sessions()
            .get_active(created.id)
            .await
            .unwrap()
            .is_none()
    );
    Ok(())
}
