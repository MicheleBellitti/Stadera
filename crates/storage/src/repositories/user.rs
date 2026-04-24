use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::StorageResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PgUserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgUserRepository<'a> {
    pub(crate) fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self))]
    pub async fn create(&self, email: &str, name: &str) -> StorageResult<Uuid> {
        let id = Uuid::now_v7();
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, name)
            VALUES ($1, $2, $3)
            "#,
            id,
            email,
            name,
        )
        .execute(self.pool)
        .await?;
        Ok(id)
    }

    #[instrument(skip(self))]
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<Option<User>> {
        let row = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    #[instrument(skip(self))]
    pub async fn get_by_email(&self, email: &str) -> StorageResult<Option<User>> {
        let row = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }
}
