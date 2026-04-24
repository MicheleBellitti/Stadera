use stadera_domain::energy::{bmr_katch_mcardle, daily_target, tdee};
use stadera_domain::units::{LeanMass, Weight};
use stadera_domain::user::ActivityLevel;

#[test]
fn bmr_with_60kg_lean_mass() {
    let lm = LeanMass::new(60.0).unwrap();
    let bmr = bmr_katch_mcardle(lm);
    // 370 + 21.6 * 60 = 1666
    assert!((bmr - 1666.0).abs() < 0.01);
}

#[test]
fn tdee_sedentary() {
    let lm = LeanMass::new(60.0).unwrap();
    let result = tdee(lm, ActivityLevel::Sedentary);
    // 1666 * 1.2 = 1999.2
    assert!((result - 1999.2).abs() < 0.01);
}

#[test]
fn tdee_very_active() {
    let lm = LeanMass::new(60.0).unwrap();
    let result = tdee(lm, ActivityLevel::VeryActive);
    // 1666 * 1.725 = 2873.85
    assert!((result - 2873.85).abs() < 0.01);
}

#[test]
fn daily_target_with_deficit() {
    let target = daily_target(2200.0, Weight::new(83.0).unwrap(), 500.0, 1.8);
    assert!((target.as_ref().unwrap().kcal - 1700.0).abs() < 0.01);
    assert!((target.as_ref().unwrap().protein_g - 149.4).abs() < 0.01);
}

#[test]
fn daily_target_zero_deficit() {
    let target = daily_target(2000.0, Weight::new(70.0).unwrap(), 0.0, 2.0);
    assert!((target.as_ref().unwrap().kcal - 2000.0).abs() < 0.01);
    assert!((target.as_ref().unwrap().protein_g - 140.0).abs() < 0.01);
}

#[test]
fn daily_target_below_safe() {
    let target = daily_target(2500.0, Weight::new(80.0).unwrap(), 1500.0, 2.0);
    assert!(target.is_err());
}
