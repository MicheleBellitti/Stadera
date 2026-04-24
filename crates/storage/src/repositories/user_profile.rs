use chrono::Utc;
use sqlx::PgPool;
use stadera_domain::UserProfile;
use tracing::instrument;
use uuid::Uuid;

use crate::error::StorageResult;
use crate::rows::{UserProfilesRow, activity_level_to_str, sex_to_str};

pub struct PgUserProfileRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgUserProfileRepository<'a> {
    pub(crate) fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self, profile))]
    pub async fn upsert(&self, user_id: Uuid, profile: &UserProfile) -> StorageResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO user_profiles
                (user_id, sex, birth_date, height_cm, activity_level, goal_weight_kg, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id) DO UPDATE SET
                sex            = EXCLUDED.sex,
                birth_date     = EXCLUDED.birth_date,
                height_cm      = EXCLUDED.height_cm,
                activity_level = EXCLUDED.activity_level,
                goal_weight_kg = EXCLUDED.goal_weight_kg,
                updated_at     = EXCLUDED.updated_at
            "#,
            user_id,
            sex_to_str(profile.sex),
            profile.birth_date,
            profile.height.value(),
            activity_level_to_str(profile.activity),
            profile.goal_weight.value(),
            Utc::now(),
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_for_user(&self, user_id: Uuid) -> StorageResult<Option<UserProfile>> {
        let row = sqlx::query_as!(
            UserProfilesRow,
            r#"
            SELECT user_id, sex, birth_date, height_cm, activity_level, goal_weight_kg, updated_at
            FROM user_profiles
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(self.pool)
        .await?;
        row.map(UserProfile::try_from).transpose()
    }
}
