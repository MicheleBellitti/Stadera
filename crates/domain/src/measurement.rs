use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::units::{BodyFatPercent, Height, LeanMass, Weight};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source{
    Withings,
    Manual,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub taken_at: DateTime<Utc>,
    pub weight: Weight,
    pub body_fat: Option<BodyFatPercent>,
    pub lean_mass: Option<LeanMass>,
    pub source: Source,
}

impl Measurement {
    pub fn new(
        taken_at: DateTime<Utc>,
        weight: Weight,
        body_fat: Option<BodyFatPercent>,
        lean_mass: Option<LeanMass>,
        source: Source,
    ) -> Self {
        Self {
            taken_at,
            weight,
            body_fat,
            lean_mass,
            source,

        }
    }

    pub fn fat_mass(&self) -> Option<f64> {
        self.body_fat
            .map(|bf| self.weight.value() * (bf.value() / 100.0))
    }

    pub fn bmi(&self, height: Height) -> f64 {
        let height_m = height.value() / 100.0;
        self.weight.value() / (height_m * height_m)
    }
}
