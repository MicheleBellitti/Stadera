//! OAuth 2.0 flow for Withings: authorization code, token exchange, refresh.
//!
//! Withings uses standard RFC 6749 with the authorization-code grant.
//! Refresh tokens have long expiry; access tokens expire in 3h (per docs).
