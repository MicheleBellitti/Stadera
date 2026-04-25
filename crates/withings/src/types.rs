//! Wire types matching Withings Health Mate API responses.
//!
//! Withings wraps every response in:
//!
//! ```json
//! { "status": 0, "body": { ... } }
//! ```
//!
//! `status == 0` is success. Non-zero codes are documented errors
//! (see <https://developer.withings.com/api-reference#tag/return-status>).
