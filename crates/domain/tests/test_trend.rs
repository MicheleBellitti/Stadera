use chrono::{NaiveDate, TimeDelta, TimeZone, Utc};
use stadera_domain::measurement::{Measurement, Source};
use stadera_domain::trend::{compute_trend, estimate_goal_date};
use stadera_domain::units::Weight;

fn make_measurement(date: NaiveDate, weight_kg: f64) -> Measurement {
    Measurement::new(
        Utc.from_utc_datetime(&date.and_hms_opt(8, 0, 0).unwrap()),
        Weight::new(weight_kg).unwrap(),
        None,
        None,
        Source::Withings,
    )
}

#[test]
fn empty_measurements() {
    let trend = compute_trend(&[]);
    assert!(trend.moving_average_7d.is_none());
    assert!(trend.weekly_delta_kg.is_none());
}

#[test]
fn fewer_than_seven_days() {
    let base = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
    let measurements = vec![
        make_measurement(base, 80.0),
        make_measurement(base + TimeDelta::days(1), 79.8),
        make_measurement(base + TimeDelta::days(2), 79.6),
    ];
    let trend = compute_trend(&measurements);
    assert!(trend.moving_average_7d.is_some());
    assert!(trend.weekly_delta_kg.is_none());
}

#[test]
fn exactly_seven_days() {
    let base = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let measurements: Vec<Measurement> = (0..7)
        .map(|i| make_measurement(base + TimeDelta::days(i), 80.0 - i as f64 * 0.2))
        .collect();
    let trend = compute_trend(&measurements);
    assert!(trend.moving_average_7d.is_some());
    assert!(trend.weekly_delta_kg.is_none());
}

#[test]
fn fourteen_days_computes_delta() {
    let base = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let measurements: Vec<Measurement> = (0..14)
        .map(|i| make_measurement(base + TimeDelta::days(i), 80.0 - i as f64 * 0.1))
        .collect();
    let trend = compute_trend(&measurements);
    assert!(trend.moving_average_7d.is_some());
    assert!(trend.weekly_delta_kg.is_some());
    assert!(trend.weekly_delta_kg.unwrap() < 0.0);
}

#[test]
fn moving_average_value() {
    let base = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let measurements = vec![
        make_measurement(base, 80.0),
        make_measurement(base + TimeDelta::days(1), 80.0),
        make_measurement(base + TimeDelta::days(2), 80.0),
    ];
    let trend = compute_trend(&measurements);
    let avg = trend.moving_average_7d.unwrap();
    assert!((avg.value() - 80.0).abs() < 0.01);
}

// — estimate_goal_date —

#[test]
fn estimate_goal_losing_weight() {
    let today = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let current = Weight::new(80.0).unwrap();
    let goal = Weight::new(75.0).unwrap();
    let result = estimate_goal_date(current, goal, -0.5, today);
    assert!(result.is_some());
    // 5 kg / 0.5 kg/week = 10 weeks = 70 days
    let expected = today + TimeDelta::days(70);
    assert_eq!(result.unwrap(), expected);
}

#[test]
fn estimate_goal_wrong_direction() {
    let today = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let current = Weight::new(80.0).unwrap();
    let goal = Weight::new(75.0).unwrap();
    // gaining weight but goal is lower → None
    assert!(estimate_goal_date(current, goal, 0.5, today).is_none());
}

#[test]
fn estimate_goal_already_at_target() {
    let today = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let w = Weight::new(75.0).unwrap();
    assert_eq!(estimate_goal_date(w, w, -0.5, today), Some(today));
}

#[test]
fn estimate_goal_zero_delta() {
    let today = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let current = Weight::new(80.0).unwrap();
    let goal = Weight::new(75.0).unwrap();
    assert!(estimate_goal_date(current, goal, 0.0, today).is_none());
}
