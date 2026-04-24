use chrono::Utc;
use stadera_domain::measurement::Measurement;
use stadera_domain::units::{BodyFatPercent, Height, LeanMass, Weight};

fn sample_full() -> Measurement {
    Measurement::new(
        Utc::now(),
        Weight::new(80.0).unwrap(),
        Some(BodyFatPercent::new(20.0).unwrap()),
        Some(LeanMass::new(64.0).unwrap()),
    )
}

#[test]
fn fat_mass_with_body_fat() {
    let m = sample_full();
    let fat = m.fat_mass().unwrap();
    assert!((fat - 16.0).abs() < 0.01);
}

#[test]
fn fat_mass_without_body_fat() {
    let m = Measurement::new(Utc::now(), Weight::new(75.0).unwrap(), None, None);
    assert!(m.fat_mass().is_none());
}

#[test]
fn bmi_calculation() {
    let m = Measurement::new(Utc::now(), Weight::new(70.0).unwrap(), None, None);
    let bmi = m.bmi(Height::new(175.0).unwrap());
    // 70 / (1.75^2) = 70 / 3.0625 ≈ 22.857
    assert!((bmi - 22.857).abs() < 0.01);
}

#[test]
fn bmi_short_person() {
    let m = Measurement::new(Utc::now(), Weight::new(60.0).unwrap(), None, None);
    let bmi = m.bmi(Height::new(150.0).unwrap());
    // 60 / (1.5^2) = 60 / 2.25 ≈ 26.667
    assert!((bmi - 26.667).abs() < 0.01);
}
