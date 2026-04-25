//! Withings → Stadera sync job.
//!
//! Steps:
//! 1. Connect Postgres.
//! 2. Look up the user by email; refuse if missing.
//! 3. Load encrypted Withings credentials; refuse if missing (run `pair` first).
//! 4. Decrypt access + refresh tokens.
//! 5. If access token is within 60s of expiry, refresh it and persist.
//! 6. Call Withings `getmeas` for the window `[now - window_days, now]`.
//! 7. Map measure groups → domain `Measurement`. Skip groups without a weight.
//! 8. `insert_or_skip_batch` with the UNIQUE constraint on
//!    `(user_id, taken_at, source)` for idempotency.
//! 9. Log totals (`fetched`, `inserted`, `skipped`).

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use stadera_domain::{BodyFatPercent, LeanMass, Measurement, Source, Weight};
use stadera_storage::{StorageContext, WithingsCredentials};
use stadera_withings::WithingsClient;
use stadera_withings::crypto::{self, Cipher};
use stadera_withings::oauth::{TokenResponse, WithingsOauth};
use stadera_withings::types::{MeasureGroup, measure_type};

/// Refresh tokens that expire within this window.
const REFRESH_BUFFER: Duration = Duration::seconds(60);

/// Entry point invoked from `main`.
pub async fn run(user_email: &str, window_days: i64) -> Result<()> {
    if window_days < 1 {
        anyhow::bail!("--window-days must be >= 1, got {window_days}");
    }

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL env var is required")?;
    let pool = PgPool::connect(&database_url)
        .await
        .context("failed to connect to Postgres")?;
    let storage = StorageContext::new(pool);

    let user = storage
        .users()
        .get_by_email(user_email)
        .await?
        .with_context(|| format!("user not found for email: {user_email}"))?;
    info!(user_id = %user.id, email = %user.email, "starting Withings sync");

    let creds = storage
        .withings_credentials()
        .get(user.id)
        .await?
        .context("no Withings credentials stored for this user — run `pair` first")?;

    let cipher = crypto::cipher_from_env()
        .context("failed to load encryption cipher (WITHINGS_TOKEN_KEY)")?;

    let access_token = decrypt_to_string(&cipher, &creds.access_token_enc, "access_token")?;
    let refresh_token = decrypt_to_string(&cipher, &creds.refresh_token_enc, "refresh_token")?;

    // Refresh proactively if near expiry.
    let access_token = if needs_refresh(creds.expires_at) {
        info!(
            expires_at = %creds.expires_at,
            "access token within {}s of expiry, refreshing",
            REFRESH_BUFFER.num_seconds(),
        );
        refresh_and_persist(&storage, &cipher, user.id, &refresh_token).await?
    } else {
        access_token
    };

    let to = Utc::now();
    let from = to - Duration::days(window_days);

    let client = WithingsClient::new().context("failed to build Withings HTTP client")?;
    let groups = client
        .get_measurements(&access_token, from, to)
        .await
        .context("failed to fetch measurements from Withings")?;
    info!(
        groups = groups.len(),
        from = %from, to = %to,
        "fetched measure groups from Withings",
    );

    let measurements: Vec<Measurement> = groups.iter().filter_map(map_group).collect();
    let mapped = measurements.len();
    let dropped = groups.len() - mapped;
    if dropped > 0 {
        warn!(dropped, "skipped groups without a usable weight measure");
    }

    let inserted = storage
        .measurements()
        .insert_or_skip_batch(user.id, &measurements)
        .await?;
    let skipped = mapped.saturating_sub(inserted);

    info!(
        groups = groups.len(),
        mapped, inserted, skipped, "sync complete",
    );
    Ok(())
}

fn needs_refresh(expires_at: DateTime<Utc>) -> bool {
    expires_at - REFRESH_BUFFER <= Utc::now()
}

fn decrypt_to_string(cipher: &Cipher, blob: &[u8], label: &'static str) -> Result<String> {
    let bytes =
        crypto::decrypt(cipher, blob).with_context(|| format!("failed to decrypt {label}"))?;
    String::from_utf8(bytes).with_context(|| format!("{label} is not valid UTF-8"))
}

async fn refresh_and_persist(
    storage: &StorageContext,
    cipher: &Cipher,
    user_id: Uuid,
    refresh_token: &str,
) -> Result<String> {
    let client_id = std::env::var("WITHINGS_CLIENT_ID")
        .context("WITHINGS_CLIENT_ID env var is required for token refresh")?;
    let client_secret = std::env::var("WITHINGS_CLIENT_SECRET")
        .context("WITHINGS_CLIENT_SECRET env var is required for token refresh")?;
    // The redirect_uri is not actually used by the refresh request, but
    // `WithingsOauth::new` requires a syntactically valid one.
    let redirect_uri = "http://localhost:7878/callback".to_string();

    let oauth = WithingsOauth::new(client_id, client_secret, redirect_uri)
        .context("failed to build WithingsOauth for refresh")?;

    let new_tokens: TokenResponse = oauth
        .refresh(refresh_token)
        .await
        .context("Withings refused refresh_token (re-pair required?)")?;

    let access_enc = crypto::encrypt(cipher, new_tokens.access_token.as_bytes())
        .context("failed to encrypt new access_token")?;
    let refresh_enc = crypto::encrypt(cipher, new_tokens.refresh_token.as_bytes())
        .context("failed to encrypt new refresh_token")?;
    let expires_at = Utc::now() + Duration::seconds(new_tokens.expires_in);

    let updated = WithingsCredentials {
        user_id,
        access_token_enc: access_enc,
        refresh_token_enc: refresh_enc,
        expires_at,
        scope: new_tokens.scope,
    };
    storage.withings_credentials().upsert(&updated).await?;
    info!("refreshed and persisted Withings tokens");

    Ok(new_tokens.access_token)
}

/// Decode a Withings `MeasureGroup` into a domain `Measurement`.
///
/// Returns `None` if:
/// - the group has no weight measurement (we cannot construct a `Measurement`),
/// - the weight value is out of the domain range,
/// - the timestamp is outside i64 range (impossible in practice).
///
/// Body-fat-percent and lean-mass are optional; if their values are out of
/// range we just drop them rather than failing the whole group.
fn map_group(g: &MeasureGroup) -> Option<Measurement> {
    let weight = find_value(g, measure_type::WEIGHT_KG).and_then(|v| Weight::new(v).ok())?;
    let taken_at = DateTime::<Utc>::from_timestamp(g.date, 0)?;

    let body_fat =
        find_value(g, measure_type::BODY_FAT_PERCENT).and_then(|v| BodyFatPercent::new(v).ok());
    let lean_mass = find_value(g, measure_type::LEAN_MASS_KG).and_then(|v| LeanMass::new(v).ok());

    Some(Measurement::new(
        taken_at,
        weight,
        body_fat,
        lean_mass,
        Source::Withings,
    ))
}

fn find_value(g: &MeasureGroup, kind: i32) -> Option<f64> {
    g.measures
        .iter()
        .find(|m| m.measure_type == kind)
        .map(|m| m.as_f64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use stadera_withings::types::Measure;

    fn measure(measure_type: i32, value: i64, unit: i32) -> Measure {
        Measure {
            value,
            measure_type,
            unit,
        }
    }

    fn group(date: i64, measures: Vec<Measure>) -> MeasureGroup {
        MeasureGroup {
            grpid: 1,
            attrib: 0,
            date,
            created: None,
            category: 1,
            deviceid: None,
            measures,
            comment: None,
        }
    }

    #[test]
    fn map_group_with_full_measures() {
        let g = group(
            1745452800,
            vec![
                measure(measure_type::WEIGHT_KG, 80000, -3),      // 80 kg
                measure(measure_type::BODY_FAT_PERCENT, 200, -1), // 20.0%
                measure(measure_type::LEAN_MASS_KG, 64000, -3),   // 64 kg
            ],
        );
        let m = map_group(&g).unwrap();
        assert_eq!(m.weight.value(), 80.0);
        assert_eq!(m.body_fat.unwrap().value(), 20.0);
        assert_eq!(m.lean_mass.unwrap().value(), 64.0);
        assert_eq!(m.source, Source::Withings);
        assert_eq!(
            m.taken_at,
            Utc.timestamp_opt(1745452800, 0).single().unwrap()
        );
    }

    #[test]
    fn map_group_without_weight_returns_none() {
        let g = group(
            1745452800,
            vec![
                measure(measure_type::BODY_FAT_PERCENT, 200, -1), // only body fat, no weight
            ],
        );
        assert!(map_group(&g).is_none());
    }

    #[test]
    fn map_group_drops_out_of_range_optionals_but_keeps_weight() {
        let g = group(
            1745452800,
            vec![
                measure(measure_type::WEIGHT_KG, 80000, -3),      // valid
                measure(measure_type::BODY_FAT_PERCENT, 1000, 0), // 1000% — out of range, drop
            ],
        );
        let m = map_group(&g).unwrap();
        assert_eq!(m.weight.value(), 80.0);
        assert!(m.body_fat.is_none(), "out-of-range body fat dropped");
    }

    #[test]
    fn needs_refresh_inside_buffer() {
        // expires 30s in the future → within 60s buffer → refresh
        let expires = Utc::now() + Duration::seconds(30);
        assert!(needs_refresh(expires));
    }

    #[test]
    fn needs_refresh_well_in_the_future() {
        // expires in 1h → no refresh
        let expires = Utc::now() + Duration::hours(1);
        assert!(!needs_refresh(expires));
    }

    #[test]
    fn needs_refresh_already_expired() {
        let expires = Utc::now() - Duration::seconds(10);
        assert!(needs_refresh(expires));
    }
}
