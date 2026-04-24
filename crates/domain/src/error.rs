use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DomainError {
    #[error("invalid weight: {value} kg (expected 10..500)")]
    InvalidWeight { value: f64 },

    #[error("invalid body fat percent: {value}% (expected 2..80)")]
    InvalidBodyFat { value: f64 },

    #[error("invalid lean mass: {value} kg")]
    InvalidLeanMass { value: f64 },

    #[error("invalid height: {value} cm (expected 50..300)")]
    InvalidHeight { value: f64 },

    #[error("value is NaN or infinite")]
    NotFinite,
    #[error("calorie target of {kcal} kcal is below the safe minimum of {min} kcal")]
    UnsafeCaloriTarget { kcal: f64, min: f64 },
    // WIP: aggiungere altri errori specifici per il dominio, es. InvalidUnit, OutOfRange, etc.
}
