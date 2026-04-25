use sqlx::PgPool;
use stadera_storage::StorageContext;
use uuid::Uuid;

#[sqlx::test]
async fn create_and_get_by_id(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let id = ctx
        .users()
        .create("alice@example.com", "Alice")
        .await
        .unwrap();

    let user = ctx.users().get_by_id(id).await.unwrap().unwrap();
    assert_eq!(user.id, id);
    assert_eq!(user.email, "alice@example.com");
    assert_eq!(user.name, "Alice");
    Ok(())
}

#[sqlx::test]
async fn get_by_email(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    ctx.users().create("bob@example.com", "Bob").await.unwrap();

    let user = ctx
        .users()
        .get_by_email("bob@example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.email, "bob@example.com");
    assert_eq!(user.name, "Bob");
    Ok(())
}

#[sqlx::test]
async fn get_by_id_non_existent_returns_none(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    let id = Uuid::now_v7();
    assert!(ctx.users().get_by_id(id).await.unwrap().is_none());
    Ok(())
}

#[sqlx::test]
async fn get_by_email_non_existent_returns_none(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    assert!(
        ctx.users()
            .get_by_email("missing@example.com")
            .await
            .unwrap()
            .is_none()
    );
    Ok(())
}

#[sqlx::test]
async fn duplicate_email_rejected(pool: PgPool) -> sqlx::Result<()> {
    let ctx = StorageContext::new(pool);
    ctx.users()
        .create("dup@example.com", "First")
        .await
        .unwrap();
    let result = ctx.users().create("dup@example.com", "Second").await;
    assert!(
        result.is_err(),
        "expected UNIQUE violation on duplicate email"
    );
    Ok(())
}
