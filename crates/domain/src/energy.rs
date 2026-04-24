use crate::error::DomainError;
use crate::units::{LeanMass, Weight};
use crate::user::ActivityLevel;

pub fn bmr_katch_mcardle(lean_mass: LeanMass) -> f64 {
    370.0 + 21.6 * lean_mass.value()
}

pub fn tdee(lean_mass: LeanMass, activity: ActivityLevel) -> f64 {
    bmr_katch_mcardle(lean_mass) * activity.multiplier()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DailyTarget {
    pub kcal: f64,
    pub protein_g: f64,
}

pub fn daily_target(
    tdee: f64,
    current_weight: Weight,
    deficit_kcal: f64,
    protein_per_kg: f64,
) -> Result<DailyTarget, DomainError> {
    let kcal = tdee - deficit_kcal;
    const MIN_KCAL: f64 = 1200.0;
    if kcal < MIN_KCAL {
        return Err(DomainError::UnsafeCaloriTarget {
            kcal,
            min: MIN_KCAL,
        });
    }

    Ok(DailyTarget {
        kcal,
        protein_g: current_weight.value() * protein_per_kg,
    })
}
