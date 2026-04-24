use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::StorageResult;

#[derive(Clone)]
pub struct WithingsCredentials {
    pub user_id: Uuid,
    pub access_token_enc: Vec<u8>,
    pub refresh_token_enc: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub scope: String,
}

pub struct PgWithingsCredentialsRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgWithingsCredentialsRepository<'a> {
    pub(crate) fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self, creds))]
    pub async fn upsert(&self, creds: &WithingsCredentials) -> StorageResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO withings_credentials
                (user_id, access_token_enc, refresh_token_enc, expires_at, scope)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id) DO UPDATE SET
                access_token_enc  = EXCLUDED.access_token_enc,
                refresh_token_enc = EXCLUDED.refresh_token_enc,
                expires_at        = EXCLUDED.expires_at,
                scope             = EXCLUDED.scope,
                updated_at        = now()
            "#,
            creds.user_id,
            creds.access_token_enc,
            creds.refresh_token_enc,
            creds.expires_at,
            creds.scope,
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get(&self, user_id: Uuid) -> StorageResult<Option<WithingsCredentials>> {
        let row = sqlx::query!(
            r#"
            SELECT user_id, access_token_enc, refresh_token_enc, expires_at, scope
            FROM withings_credentials
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(|r| WithingsCredentials {
            user_id: r.user_id,
            access_token_enc: r.access_token_enc,
            refresh_token_enc: r.refresh_token_enc,
            expires_at: r.expires_at,
            scope: r.scope,
        }))
    }

    #[instrument(skip(self))]
    pub async fn delete(&self, user_id: Uuid) -> StorageResult<()> {
        sqlx::query!(
            "DELETE FROM withings_credentials WHERE user_id = $1",
            user_id
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
