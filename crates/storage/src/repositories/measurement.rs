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

    /// Atomic batch insert: all measurements succeed or none are persisted.
    /// Wraps the loop in a transaction so a mid-batch failure rolls back
    /// every prior insert in the same call — safe to retry the full batch
    /// without creating duplicates (new UUIDs would otherwise be generated
    /// for already-inserted rows).
    #[instrument(skip(self, measurements))]
    pub async fn insert_batch(
        &self,
        user_id: Uuid,
        measurements: &[Measurement],
    ) -> StorageResult<Vec<Uuid>> {
        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::with_capacity(measurements.len());
        for m in measurements {
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
            .execute(&mut *tx)
            .await?;
            ids.push(id);
        }
        tx.commit().await?;
        Ok(ids)
    }

    /// Idempotent batch insert: rows already present (matched on the UNIQUE
    /// `(user_id, taken_at, source)` key) are silently skipped. Returns the
    /// number of rows actually inserted (`total - duplicates`). Wrapped in a
    /// transaction for the same atomicity guarantee as [`Self::insert_batch`].
    ///
    /// This is the method the Withings sync job uses: re-running the same
    /// 7-day window leaves the database unchanged for already-known
    /// measurements, while picking up genuinely new ones.
    #[instrument(skip(self, measurements))]
    pub async fn insert_or_skip_batch(
        &self,
        user_id: Uuid,
        measurements: &[Measurement],
    ) -> StorageResult<usize> {
        let mut tx = self.pool.begin().await?;
        let mut inserted = 0usize;
        for m in measurements {
            let id = Uuid::now_v7();
            let result = sqlx::query!(
                r#"
                INSERT INTO measurements
                    (id, user_id, taken_at, weight_kg, body_fat_percent, lean_mass_kg, source)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (user_id, taken_at, source) DO NOTHING
                "#,
                id,
                user_id,
                m.taken_at,
                m.weight.value(),
                m.body_fat.map(|bf| bf.value()),
                m.lean_mass.map(|lm| lm.value()),
                source_to_str(m.source),
            )
            .execute(&mut *tx)
            .await?;
            inserted += result.rows_affected() as usize;
        }
        tx.commit().await?;
        Ok(inserted)
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
