use chrono::Utc;
use stadera_domain::measurement::{Measurement, Source};
use stadera_domain::units::{BodyFatPercent, Height, LeanMass, Weight};

fn sample_full() -> Measurement {
    Measurement::new(
        Utc::now(),
        Weight::new(80.0).unwrap(),
        Some(BodyFatPercent::new(20.0).unwrap()),
        Some(LeanMass::new(64.0).unwrap()),
        Source::Withings,
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
    let m = Measurement::new(
        Utc::now(),
        Weight::new(75.0).unwrap(),
        None,
        None,
        Source::Manual,
    );
    assert!(m.fat_mass().is_none());
}

#[test]
fn bmi_calculation() {
    let m = Measurement::new(
        Utc::now(),
        Weight::new(70.0).unwrap(),
        None,
        None,
        Source::Withings,
    );
    let bmi = m.bmi(Height::new(175.0).unwrap());
    // 70 / (1.75^2) = 70 / 3.0625 ≈ 22.857
    assert!((bmi - 22.857).abs() < 0.01);
}

#[test]
fn bmi_short_person() {
    let m = Measurement::new(
        Utc::now(),
        Weight::new(60.0).unwrap(),
        None,
        None,
        Source::Manual,
    );
    let bmi = m.bmi(Height::new(150.0).unwrap());
    // 60 / (1.5^2) = 60 / 2.25 ≈ 26.667
    assert!((bmi - 26.667).abs() < 0.01);
}

#[test]
fn source_serde_snake_case() {
    let json = serde_json::to_string(&Source::Withings).unwrap();
    assert_eq!(json, "\"withings\"");

    let json = serde_json::to_string(&Source::Manual).unwrap();
    assert_eq!(json, "\"manual\"");

    let deserialized: Source = serde_json::from_str("\"withings\"").unwrap();
    assert_eq!(deserialized, Source::Withings);
}
