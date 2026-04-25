use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::StorageResult;

/// A server-side session row.
///
/// Created on successful Google OAuth callback; deleted on logout or by a
/// (future) cleanup job that removes rows where `expires_at <= now()`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

pub struct PgSessionRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgSessionRepository<'a> {
    pub(crate) fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new session for `user_id`. The id is generated server-side
    /// as a `Uuid::now_v7` so we never trust a client-supplied id.
    #[instrument(skip(self))]
    pub async fn create(&self, user_id: Uuid, expires_at: DateTime<Utc>) -> StorageResult<Session> {
        let id = Uuid::now_v7();
        let row = sqlx::query_as!(
            Session,
            r#"
            INSERT INTO sessions (id, user_id, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, expires_at, created_at, last_seen_at
            "#,
            id,
            user_id,
            expires_at,
        )
        .fetch_one(self.pool)
        .await?;
        Ok(row)
    }

    /// Look up an active (non-expired) session by id.
    ///
    /// Returns `None` if the row does not exist or has expired.
    #[instrument(skip(self))]
    pub async fn get_active(&self, id: Uuid) -> StorageResult<Option<Session>> {
        let row = sqlx::query_as!(
            Session,
            r#"
            SELECT id, user_id, expires_at, created_at, last_seen_at
            FROM sessions
            WHERE id = $1 AND expires_at > now()
            "#,
            id,
        )
        .fetch_optional(self.pool)
        .await?;
        Ok(row)
    }

    /// Update `last_seen_at` to `now()`. Best-effort, used by the auth
    /// middleware to track activity.
    #[instrument(skip(self))]
    pub async fn touch(&self, id: Uuid) -> StorageResult<()> {
        sqlx::query!("UPDATE sessions SET last_seen_at = now() WHERE id = $1", id,)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Delete a single session (logout).
    #[instrument(skip(self))]
    pub async fn delete(&self, id: Uuid) -> StorageResult<()> {
        sqlx::query!("DELETE FROM sessions WHERE id = $1", id)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}
