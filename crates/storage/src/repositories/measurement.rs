use chrono::{DateTime, Utc};
use sqlx::PgPool;
use stadera_domain::Measurement;
use tracing::instrument;
use uuid::Uuid;

use crate::error::StorageResult;
use crate::rows::{MeasurementRow, source_to_str};

pub struct PgMeasurementRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgMeasurementRepository<'a> {
    pub(crate) fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self, m))]
    pub async fn insert(&self, user_id: Uuid, m: &Measurement) -> StorageResult<Uuid> {
        let id = Uuid::now_v7();
        sqlx::query!(
            r#"
            INSERT INTO measurements
                (id, user_id, taken_at, weight_kg, body_fat_percent, lean_mass_kg, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            id,
            user_id,
            m.taken_at,
            m.weight.value(),
            m.body_fat.map(|bf| bf.value()),
            m.lean_mass.map(|lm| lm.value()),
            source_to_str(m.source),
        )
        .execute(self.pool)
        .await?;
        Ok(id)
    }

    #[instrument(skip(self, measurements))]
    pub async fn insert_batch(
        &self,
        user_id: Uuid,
        measurements: &[Measurement],
    ) -> StorageResult<Vec<Uuid>> {
        let mut ids = Vec::with_capacity(measurements.len());
        for m in measurements {
            ids.push(self.insert(user_id, m).await?);
        }
        Ok(ids)
    }

    #[instrument(skip(self))]
    pub async fn get_by_id(&self, id: Uuid) -> StorageResult<Option<Measurement>> {
        let row = sqlx::query_as!(
            MeasurementRow,
            r#"
            SELECT id, user_id, taken_at, weight_kg, body_fat_percent, lean_mass_kg, source, created_at
            FROM measurements
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool)
        .await?;
        row.map(Measurement::try_from).transpose()
    }

    #[instrument(skip(self))]
    pub async fn list_for_user_between(
        &self,
        user_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> StorageResult<Vec<Measurement>> {
        let rows = sqlx::query_as!(
            MeasurementRow,
            r#"
            SELECT id, user_id, taken_at, weight_kg, body_fat_percent, lean_mass_kg, source, created_at
            FROM measurements
            WHERE user_id = $1 AND taken_at >= $2 AND taken_at < $3
            ORDER BY taken_at ASC
            "#,
            user_id, from, to
        )
        .fetch_all(self.pool)
        .await?;
        rows.into_iter().map(Measurement::try_from).collect()
    }

    #[instrument(skip(self))]
    pub async fn list_for_user_latest(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> StorageResult<Vec<Measurement>> {
        let rows = sqlx::query_as!(
            MeasurementRow,
            r#"
            SELECT id, user_id, taken_at, weight_kg, body_fat_percent, lean_mass_kg, source, created_at
            FROM measurements
            WHERE user_id = $1
            ORDER BY taken_at DESC
            LIMIT $2
            "#,
            user_id, limit
        )
        .fetch_all(self.pool)
        .await?;
        rows.into_iter().map(Measurement::try_from).collect()
    }

    #[instrument(skip(self))]
    pub async fn latest_for_user(&self, user_id: Uuid) -> StorageResult<Option<Measurement>> {
        let row = sqlx::query_as!(
            MeasurementRow,
            r#"
            SELECT id, user_id, taken_at, weight_kg, body_fat_percent, lean_mass_kg, source, created_at
            FROM measurements
            WHERE user_id = $1
            ORDER BY taken_at DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(self.pool)
        .await?;
        row.map(Measurement::try_from).transpose()
    }
}
