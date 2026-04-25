//! One-shot CLI tool for first-time OAuth pairing with Withings.
//!
//! Flow:
//! 1. Print the Withings authorization URL.
//! 2. Open the user's browser at it.
//! 3. Listen on `http://localhost:7878/callback` for the redirect with `?code=...`.
//! 4. Exchange the code for tokens via Withings token endpoint.
//! 5. Encrypt tokens and persist them via `stadera-storage`.
//!
//! Run once during initial setup; thereafter the cron sync uses the stored refresh token.

fn main() {
    eprintln!(
        "stadera-withings pair: not yet implemented. \
         Will be filled in by feat(withings) work in M4."
    );
    std::process::exit(1);
}
