use proptest::prelude::*;
use stadera_domain::error::DomainError;
use stadera_domain::units::{BodyFatPercent, Height, LeanMass, Weight};

// — Weight —

#[test]
fn weight_accepts_plausible_value() {
    let w = Weight::new(75.3).unwrap();
    assert!((w.value() - 75.3).abs() < f64::EPSILON);
}

#[test]
fn weight_rejects_nan() {
    assert_eq!(Weight::new(f64::NAN), Err(DomainError::NotFinite));
}

#[test]
fn weight_rejects_negative() {
    assert!(Weight::new(-5.0).is_err());
}

#[test]
fn weight_rejects_above_limit() {
    assert!(Weight::new(501.0).is_err());
}

#[test]
fn weight_boundary_values() {
    assert!(Weight::new(10.0).is_ok());
    assert!(Weight::new(500.0).is_ok());
    assert!(Weight::new(9.9).is_err());
}

#[test]
fn weight_display() {
    assert_eq!(Weight::new(75.3).unwrap().to_string(), "75.3 kg");
}

#[test]
fn weight_try_from() {
    let w: Result<Weight, _> = 80.0_f64.try_into();
    assert!(w.is_ok());
}

#[test]
fn weight_deserialize_rejects_invalid() {
    assert!(serde_json::from_str::<Weight>("-10.0").is_err());
    assert!(serde_json::from_str::<Weight>("600.0").is_err());
    assert!(serde_json::from_str::<Weight>("null").is_err());
}

// — BodyFatPercent —

#[test]
fn body_fat_accepts_plausible_value() {
    let bf = BodyFatPercent::new(15.0).unwrap();
    assert!((bf.value() - 15.0).abs() < f64::EPSILON);
}

#[test]
fn body_fat_rejects_below_minimum() {
    assert!(BodyFatPercent::new(1.0).is_err());
}

#[test]
fn body_fat_rejects_above_limit() {
    assert!(BodyFatPercent::new(81.0).is_err());
}

#[test]
fn body_fat_rejects_infinity() {
    assert_eq!(
        BodyFatPercent::new(f64::INFINITY),
        Err(DomainError::NotFinite)
    );
}

#[test]
fn body_fat_display() {
    assert_eq!(BodyFatPercent::new(15.0).unwrap().to_string(), "15.0%");
}

#[test]
fn body_fat_deserialize_rejects_invalid() {
    assert!(serde_json::from_str::<BodyFatPercent>("-10.0").is_err());
    assert!(serde_json::from_str::<BodyFatPercent>("600.0").is_err());
    assert!(serde_json::from_str::<BodyFatPercent>("null").is_err());
}

// — LeanMass —

#[test]
fn lean_mass_accepts_plausible_value() {
    let lm = LeanMass::new(60.0).unwrap();
    assert!((lm.value() - 60.0).abs() < f64::EPSILON);
}

#[test]
fn lean_mass_rejects_nan() {
    assert_eq!(LeanMass::new(f64::NAN), Err(DomainError::NotFinite));
}

#[test]
fn lean_mass_rejects_below_minimum() {
    assert!(LeanMass::new(1.0).is_err());
}

#[test]
fn lean_mass_display() {
    assert_eq!(LeanMass::new(60.0).unwrap().to_string(), "60.0 kg");
}

#[test]
fn lean_mass_deserialize_rejects_invalid() {
    assert!(serde_json::from_str::<LeanMass>("-10.0").is_err());
    assert!(serde_json::from_str::<LeanMass>("600.0").is_err());
    assert!(serde_json::from_str::<LeanMass>("null").is_err());
}

// — Height —

#[test]
fn height_accepts_plausible_value() {
    let h = Height::new(175.0).unwrap();
    assert!((h.value() - 175.0).abs() < f64::EPSILON);
}

#[test]
fn height_rejects_below_minimum() {
    assert!(Height::new(30.0).is_err());
}

#[test]
fn height_rejects_above_limit() {
    assert!(Height::new(301.0).is_err());
}

#[test]
fn height_display() {
    assert_eq!(Height::new(175.0).unwrap().to_string(), "175.0 cm");
}

#[test]
fn height_deserialize_rejects_invalid() {
    assert!(serde_json::from_str::<Height>("-10.0").is_err());
    assert!(serde_json::from_str::<Height>("600.0").is_err());
    assert!(serde_json::from_str::<Height>("null").is_err());
}

// — Property tests —

proptest! {
    #[test]
    fn weight_roundtrip(v in 10.0f64..=500.0) {
        let w = Weight::new(v).unwrap();
        prop_assert!((w.value() - v).abs() < f64::EPSILON);
    }

    #[test]
    fn body_fat_roundtrip(v in 2.0f64..=80.0) {
        let bf = BodyFatPercent::new(v).unwrap();
        prop_assert!((bf.value() - v).abs() < f64::EPSILON);
    }

    #[test]
    fn height_roundtrip(v in 50.0f64..=300.0) {
        let h = Height::new(v).unwrap();
        prop_assert!((h.value() - v).abs() < f64::EPSILON);
    }
}
