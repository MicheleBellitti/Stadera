//! Health Mate API client.
//!
//! Wraps `reqwest`, adds bearer-token auth, and decodes Withings' envelope
//! (`{ "status": ..., "body": ..., "error": ... }`) for each endpoint.
//!
//! Endpoints implemented:
//! - `POST /measure` (action=getmeas) — list weight/body-fat/lean-mass
//!   measurements for the authenticated user

use chrono::{DateTime, Utc};

use crate::error::{WithingsError, WithingsResult};
use crate::types::{ApiEnvelope, GetMeasBody, MeasureGroup, measure_type};

const DEFAULT_BASE_URL: &str = "https://wbsapi.withings.net";

/// Health Mate API client.
pub struct WithingsClient {
    base_url: String,
    http: reqwest::Client,
}

impl WithingsClient {
    /// Construct with the production Withings base URL.
    pub fn new() -> WithingsResult<Self> {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    /// Construct with an explicit base URL (used by tests with mock servers).
    pub fn with_base_url(base_url: String) -> WithingsResult<Self> {
        Ok(Self {
            base_url,
            http: reqwest::Client::builder()
                .build()
                .map_err(WithingsError::Http)?,
        })
    }

    /// Fetch measurement groups for the authenticated user, in `[from, to]`
    /// (Withings interprets `startdate`/`enddate` as inclusive Unix seconds).
    ///
    /// `access_token` is the bearer token (already decrypted by the caller).
    /// Only weight, lean mass, and body-fat-percent measurement types are
    /// requested (`meastypes=1,5,6`).
    pub async fn get_measurements(
        &self,
        access_token: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> WithingsResult<Vec<MeasureGroup>> {
        let meastypes = format!(
            "{},{},{}",
            measure_type::WEIGHT_KG,
            measure_type::LEAN_MASS_KG,
            measure_type::BODY_FAT_PERCENT,
        );
        let startdate = from.timestamp().to_string();
        let enddate = to.timestamp().to_string();

        let envelope: ApiEnvelope<GetMeasBody> = self
            .http
            .post(format!("{}/measure", self.base_url))
            .bearer_auth(access_token)
            .form(&[
                ("action", "getmeas"),
                ("meastypes", meastypes.as_str()),
                ("category", "1"), // 1 = real measurements (exclude objectives)
                ("startdate", startdate.as_str()),
                ("enddate", enddate.as_str()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if !envelope.is_success() {
            return Err(WithingsError::Api {
                status: envelope.status,
                message: envelope.error.unwrap_or_default(),
            });
        }
        let body = envelope
            .body
            .ok_or_else(|| WithingsError::UnexpectedResponse("status=0 but body is null".into()))?;
        Ok(body.measuregrps)
    }
}
