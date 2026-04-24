use chrono::{DateTime, NaiveDate, Utc};
use sqlx::FromRow;
use stadera_domain::{
    ActivityLevel, BodyFatPercent, Height, LeanMass, Measurement, Sex, Source, UserProfile, Weight,
};
use uuid::Uuid;

use crate::error::StorageError;

#[derive(FromRow)]
#[allow(dead_code)] // id/user_id/created_at mirror the DB schema for query_as! matching; not used by the domain conversion.
pub(crate) struct MeasurementRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub taken_at: DateTime<Utc>,
    pub weight_kg: f64,
    pub body_fat_percent: Option<f64>,
    pub lean_mass_kg: Option<f64>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<MeasurementRow> for Measurement {
    type Error = StorageError;

    fn try_from(r: MeasurementRow) -> Result<Self, Self::Error> {
        let weight = Weight::new(r.weight_kg)
            .map_err(|e| StorageError::corruption("measurements", e.to_string()))?;

        let body_fat = r
            .body_fat_percent
            .map(BodyFatPercent::new)
            .transpose()
            .map_err(|e| StorageError::corruption("measurements", e.to_string()))?;

        let lean_mass = r
            .lean_mass_kg
            .map(LeanMass::new)
            .transpose()
            .map_err(|e| StorageError::corruption("measurements", e.to_string()))?;

        let source = match r.source.as_str() {
            "withings" => Source::Withings,
            "manual" => Source::Manual,
            other => {
                return Err(StorageError::corruption(
                    "measurements",
                    format!("invalid source: {other}"),
                ));
            }
        };

        Ok(Measurement::new(
            r.taken_at, weight, body_fat, lean_mass, source,
        ))
    }
}

#[derive(FromRow)]
#[allow(dead_code)] // user_id/updated_at mirror the DB schema; not part of the UserProfile domain type.
pub(crate) struct UserProfilesRow {
    pub user_id: Uuid,
    pub sex: String,
    pub birth_date: NaiveDate,
    pub height_cm: f64,
    pub activity_level: String,
    pub goal_weight_kg: f64,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<UserProfilesRow> for UserProfile {
    type Error = StorageError;

    fn try_from(r: UserProfilesRow) -> Result<Self, Self::Error> {
        let sex = match r.sex.as_str() {
            "male" => Sex::Male,
            "female" => Sex::Female,
            other => {
                return Err(StorageError::corruption(
                    "user_profiles",
                    format!("invalid sex: {other}"),
                ));
            }
        };

        let activity = match r.activity_level.as_str() {
            "sedentary" => ActivityLevel::Sedentary,
            "lightly_active" => ActivityLevel::LightlyActive,
            "moderately_active" => ActivityLevel::ModeratelyActive,
            "very_active" => ActivityLevel::VeryActive,
            other => {
                return Err(StorageError::corruption(
                    "user_profiles",
                    format!("invalid activity_level: {other}"),
                ));
            }
        };

        let height = Height::new(r.height_cm)
            .map_err(|e| StorageError::corruption("user_profiles", e.to_string()))?;

        let goal_weight = Weight::new(r.goal_weight_kg)
            .map_err(|e| StorageError::corruption("user_profiles", e.to_string()))?;

        Ok(UserProfile {
            birth_date: r.birth_date,
            sex,
            height,
            activity,
            goal_weight,
        })
    }
}

pub(crate) fn source_to_str(s: Source) -> &'static str {
    match s {
        Source::Withings => "withings",
        Source::Manual => "manual",
    }
}

pub(crate) fn sex_to_str(s: Sex) -> &'static str {
    match s {
        Sex::Male => "male",
        Sex::Female => "female",
    }
}

pub(crate) fn activity_level_to_str(a: ActivityLevel) -> &'static str {
    match a {
        ActivityLevel::Sedentary => "sedentary",
        ActivityLevel::LightlyActive => "lightly_active",
        ActivityLevel::ModeratelyActive => "moderately_active",
        ActivityLevel::VeryActive => "very_active",
    }
}
