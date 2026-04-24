use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Weight in kg.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64")]
pub struct Weight(f64);

impl Weight {
    pub fn new(value: f64) -> Result<Self, DomainError> {
        if !value.is_finite() {
            return Err(DomainError::NotFinite);
        }
        if !(10.0..=500.0).contains(&value) {
            return Err(DomainError::InvalidWeight { value });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Weight {
    type Error = DomainError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for Weight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} kg", self.0)
    }
}

/// Body fat percentage (2.0–80.0).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64")]
pub struct BodyFatPercent(f64);

impl BodyFatPercent {
    pub fn new(value: f64) -> Result<Self, DomainError> {
        if !value.is_finite() {
            return Err(DomainError::NotFinite);
        }
        if !(2.0..=80.0).contains(&value) {
            return Err(DomainError::InvalidBodyFat { value });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for BodyFatPercent {
    type Error = DomainError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for BodyFatPercent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}%", self.0)
    }
}

/// Lean mass in kg.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64")]
pub struct LeanMass(f64);

impl LeanMass {
    pub fn new(value: f64) -> Result<Self, DomainError> {
        if !value.is_finite() {
            return Err(DomainError::NotFinite);
        }
        if !(2.0..=300.0).contains(&value) {
            return Err(DomainError::InvalidLeanMass { value });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for LeanMass {
    type Error = DomainError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for LeanMass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} kg", self.0)
    }
}

/// Height in cm.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64")]
pub struct Height(f64);

impl Height {
    pub fn new(value: f64) -> Result<Self, DomainError> {
        if !value.is_finite() {
            return Err(DomainError::NotFinite);
        }
        if !(50.0..=300.0).contains(&value) {
            return Err(DomainError::InvalidHeight { value });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Height {
    type Error = DomainError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for Height {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} cm", self.0)
    }
}
