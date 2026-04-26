//! Wire types matching Withings Health Mate API responses.
//!
//! Withings wraps every response in an envelope:
//!
//! ```json
//! { "status": 0, "body": { ... }, "error": null }
//! ```
//!
//! `status == 0` is success; non-zero codes are documented errors
//! (see <https://developer.withings.com/api-reference#tag/return-status>).
//! Concrete body types are added per-endpoint as the client implementation grows.

use serde::Deserialize;

/// Generic envelope returned by every Withings endpoint.
#[derive(Debug, Deserialize)]
pub struct ApiEnvelope<T> {
    pub status: i64,
    /// Present only when `status == 0`.
    pub body: Option<T>,
    /// Present only when `status != 0`.
    pub error: Option<String>,
}

impl<T> ApiEnvelope<T> {
    /// Returns true when the response represents a successful Withings call.
    pub fn is_success(&self) -> bool {
        self.status == 0
    }
}

// ---- /measure?action=getmeas response ----------------------------------

/// Body of `/measure?action=getmeas`.
///
/// Each `measuregrp` represents a single weighing event with one or more
/// individual `measures` (weight, body fat percent, lean mass, …).
///
/// `updatetime` is documented as always-present but Withings omits it in
/// practice when `measuregrps` is empty (observed on freshly paired
/// accounts with zero readings). It's also never read in this codebase,
/// so we accept it as `Option<i64>` and move on.
#[derive(Debug, Deserialize)]
pub struct GetMeasBody {
    #[serde(default)]
    pub updatetime: Option<i64>,
    #[serde(default)]
    pub timezone: Option<String>,
    pub measuregrps: Vec<MeasureGroup>,
}

/// A single weighing event.
#[derive(Debug, Deserialize)]
pub struct MeasureGroup {
    pub grpid: i64,
    /// Attribution: 0 = device, 1 = ambiguous, 2 = manual user creation, 4 = manual user creation during setup, 5 = measure created by app, 7 = measure auto-created
    pub attrib: i32,
    /// Unix timestamp (seconds) when the measurement was taken on the device.
    pub date: i64,
    /// Unix timestamp (seconds) when Withings server stored it.
    #[serde(default)]
    pub created: Option<i64>,
    /// 1 = real measurement, 2 = user objective.
    pub category: i32,
    #[serde(default)]
    pub deviceid: Option<String>,
    pub measures: Vec<Measure>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// One scalar measurement within a `MeasureGroup`.
///
/// The actual value is `value * 10^unit` (e.g. value=80000, unit=-3 → 80.000 kg).
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Measure {
    pub value: i64,
    /// Measurement type. Subset relevant for Stadera:
    /// 1 = weight (kg), 5 = lean mass (kg), 6 = body fat percent.
    /// Full list: <https://developer.withings.com/api-reference#tag/measure>.
    #[serde(rename = "type")]
    pub measure_type: i32,
    pub unit: i32,
}

impl Measure {
    /// Decode `value * 10^unit` into a floating-point quantity.
    pub fn as_f64(self) -> f64 {
        (self.value as f64) * 10f64.powi(self.unit)
    }
}

/// Withings measure type codes used by Stadera.
pub mod measure_type {
    pub const WEIGHT_KG: i32 = 1;
    pub const LEAN_MASS_KG: i32 = 5;
    pub const BODY_FAT_PERCENT: i32 = 6;
}
