//! HTTP request/response shapes shared across handlers.
//!
//! Domain types (`Weight`, `Sex`, `Measurement`, …) are not serialized
//! directly to JSON — they would either leak internal representations or
//! force consumers to know about newtypes. We translate them through
//! plain DTOs with `f64` and snake_case strings, which is what the
//! frontend expects.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use stadera_domain::{ActivityLevel, DailyTarget, Measurement, Sex, UserProfile};
use uuid::Uuid;

#[derive(Serialize)]
pub struct UserView {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

/// Wire shape of a single measurement.
#[derive(Serialize)]
pub struct MeasurementView {
    pub taken_at: DateTime<Utc>,
    pub weight_kg: f64,
    pub body_fat_percent: Option<f64>,
    pub lean_mass_kg: Option<f64>,
    pub source: &'static str,
}

impl From<&Measurement> for MeasurementView {
    fn from(m: &Measurement) -> Self {
        Self {
            taken_at: m.taken_at,
            weight_kg: m.weight.value(),
            body_fat_percent: m.body_fat.map(|b| b.value()),
            lean_mass_kg: m.lean_mass.map(|l| l.value()),
            source: source_to_str(m.source),
        }
    }
}

#[derive(Serialize)]
pub struct DailyTargetView {
    pub kcal: f64,
    pub protein_g: f64,
}

impl From<DailyTarget> for DailyTargetView {
    fn from(t: DailyTarget) -> Self {
        Self {
            kcal: t.kcal,
            protein_g: t.protein_g,
        }
    }
}

#[derive(Serialize)]
pub struct ProfileView {
    pub birth_date: NaiveDate,
    pub sex: &'static str,
    pub height_cm: f64,
    pub activity_level: &'static str,
    pub goal_weight_kg: f64,
}

impl From<&UserProfile> for ProfileView {
    fn from(p: &UserProfile) -> Self {
        Self {
            birth_date: p.birth_date,
            sex: sex_to_str(p.sex),
            height_cm: p.height.value(),
            activity_level: activity_level_to_str(p.activity),
            goal_weight_kg: p.goal_weight.value(),
        }
    }
}

// ---- string helpers --------------------------------------------------

pub fn sex_to_str(s: Sex) -> &'static str {
    match s {
        Sex::Male => "male",
        Sex::Female => "female",
    }
}

pub fn activity_level_to_str(a: ActivityLevel) -> &'static str {
    match a {
        ActivityLevel::Sedentary => "sedentary",
        ActivityLevel::LightlyActive => "lightly_active",
        ActivityLevel::ModeratelyActive => "moderately_active",
        ActivityLevel::VeryActive => "very_active",
    }
}

pub fn parse_sex(s: &str) -> Option<Sex> {
    match s {
        "male" => Some(Sex::Male),
        "female" => Some(Sex::Female),
        _ => None,
    }
}

pub fn parse_activity_level(s: &str) -> Option<ActivityLevel> {
    match s {
        "sedentary" => Some(ActivityLevel::Sedentary),
        "lightly_active" => Some(ActivityLevel::LightlyActive),
        "moderately_active" => Some(ActivityLevel::ModeratelyActive),
        "very_active" => Some(ActivityLevel::VeryActive),
        _ => None,
    }
}

fn source_to_str(s: stadera_domain::Source) -> &'static str {
    match s {
        stadera_domain::Source::Withings => "withings",
        stadera_domain::Source::Manual => "manual",
    }
}
