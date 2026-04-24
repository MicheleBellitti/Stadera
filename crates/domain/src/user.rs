use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::units::{Height, Weight};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Sex {
    Male,
    Female,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ActivityLevel {
    Sedentary,
    LightlyActive,
    ModeratelyActive,
    VeryActive,
}

impl ActivityLevel {
    pub fn multiplier(&self) -> f64 {
        match self {
            Self::Sedentary => 1.2,
            Self::LightlyActive => 1.375,
            Self::ModeratelyActive => 1.55,
            Self::VeryActive => 1.725,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub birth_date: NaiveDate,
    pub sex: Sex,
    pub height: Height,
    pub activity: ActivityLevel,
    pub goal_weight: Weight,
}

impl UserProfile {
    pub fn age(&self, today: NaiveDate) -> u32 {
        let mut age = today.year() - self.birth_date.year();
        if (today.month(), today.day()) < (self.birth_date.month(), self.birth_date.day()) {
            age -= 1;
        }
        age.max(0) as u32
    }
}
