use chrono::{NaiveDate, TimeDelta};

use crate::measurement::Measurement;
use crate::units::Weight;

#[derive(Debug)]
pub struct WeightTrend {
    pub moving_average_7d: Option<Weight>,
    pub weekly_delta_kg: Option<f64>,
}

pub fn compute_trend(measurements: &[Measurement]) -> WeightTrend {
    if measurements.is_empty() {
        return WeightTrend {
            moving_average_7d: None,
            weekly_delta_kg: None,
        };
    }

    let mut sorted: Vec<&Measurement> = measurements.iter().collect();
    sorted.sort_by_key(|m| m.taken_at);

    let Some(latest) = sorted.last() else {
        return WeightTrend {
            moving_average_7d: None,
            weekly_delta_kg: None,
        };
    };

    let latest_date = latest.taken_at.date_naive();
    let seven_days_ago = latest_date - TimeDelta::days(7);
    let fourteen_days_ago = latest_date - TimeDelta::days(14);

    let recent: Vec<f64> = sorted
        .iter()
        .filter(|m| m.taken_at.date_naive() > seven_days_ago)
        .map(|m| m.weight.value())
        .collect();

    let previous: Vec<f64> = sorted
        .iter()
        .filter(|m| {
            let d = m.taken_at.date_naive();
            d > fourteen_days_ago && d <= seven_days_ago
        })
        .map(|m| m.weight.value())
        .collect();

    let avg_recent = if recent.is_empty() {
        None
    } else {
        Some(recent.iter().sum::<f64>() / recent.len() as f64)
    };

    let avg_previous = if previous.is_empty() {
        None
    } else {
        Some(previous.iter().sum::<f64>() / previous.len() as f64)
    };

    let moving_average_7d = avg_recent.and_then(|v| Weight::new(v).ok());

    let weekly_delta_kg = match (avg_recent, avg_previous) {
        (Some(r), Some(p)) => Some(r - p),
        _ => None,
    };

    WeightTrend {
        moving_average_7d,
        weekly_delta_kg,
    }
}

pub fn estimate_goal_date(
    current: Weight,
    goal: Weight,
    weekly_delta_kg: f64,
    today: NaiveDate,
) -> Option<NaiveDate> {
    let diff = goal.value() - current.value();

    if diff.abs() < 0.01 {
        return Some(today);
    }
    if weekly_delta_kg.abs() < f64::EPSILON {
        return None;
    }
    if diff.signum() != weekly_delta_kg.signum() {
        return None;
    }

    let weeks = diff / weekly_delta_kg;
    let days = (weeks * 7.0).ceil() as i64;
    today.checked_add_signed(TimeDelta::days(days))
}
