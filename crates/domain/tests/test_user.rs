use chrono::NaiveDate;
use stadera_domain::units::{Height, Weight};
use stadera_domain::user::{ActivityLevel, Sex, UserProfile};

fn sample_profile() -> UserProfile {
    UserProfile {
        birth_date: NaiveDate::from_ymd_opt(1990, 6, 15).unwrap(),
        sex: Sex::Male,
        height: Height::new(175.0).unwrap(),
        activity: ActivityLevel::ModeratelyActive,
        goal_weight: Weight::new(75.0).unwrap(),
    }
}

#[test]
fn age_on_birthday() {
    let p = sample_profile();
    let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    assert_eq!(p.age(today), 34);
}

#[test]
fn age_before_birthday_this_year() {
    let p = sample_profile();
    let today = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
    assert_eq!(p.age(today), 33);
}

#[test]
fn age_after_birthday_this_year() {
    let p = sample_profile();
    let today = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    assert_eq!(p.age(today), 34);
}

#[test]
fn activity_multipliers() {
    assert!((ActivityLevel::Sedentary.multiplier() - 1.2).abs() < f64::EPSILON);
    assert!((ActivityLevel::LightlyActive.multiplier() - 1.375).abs() < f64::EPSILON);
    assert!((ActivityLevel::ModeratelyActive.multiplier() - 1.55).abs() < f64::EPSILON);
    assert!((ActivityLevel::VeryActive.multiplier() - 1.725).abs() < f64::EPSILON);
}
