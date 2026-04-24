//! Stadera domain: business logic for weight tracking and nutrition.
//!
//! Pure functions and strongly-typed values. No I/O.

pub mod energy;
pub mod error;
pub mod measurement;
pub mod trend;
pub mod units;
pub mod user;

pub use energy::DailyTarget;
pub use error::DomainError;
pub use measurement::{Measurement, Source};
pub use trend::WeightTrend;
pub use units::{BodyFatPercent, Height, LeanMass, Weight};
pub use user::{ActivityLevel, Sex, UserProfile};
