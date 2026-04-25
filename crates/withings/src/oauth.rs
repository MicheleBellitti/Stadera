//! OAuth 2.0 flow for Withings.
//!
//! Uses the `oauth2` crate for type-safe wrappers (`ClientId`, `ClientSecret`,
//! `RedirectUrl`, `CsrfToken`) and authorization-URL building, but performs
//! the actual token exchange and refresh via raw `reqwest`. The reason is
//! that Withings wraps the token response in their `{status, body}` envelope
//! (see [`crate::types::ApiEnvelope`]) instead of returning a plain RFC 6749
//! body — bypassing `oauth2`'s built-in token handler avoids fighting the
//! crate over a single-endpoint quirk.
//!
//! Standards-compliant OAuth providers (e.g. Google in M5) can use
//! `oauth2`'s `request_async` end-to-end without this workaround.

use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenUrl};
use serde::Deserialize;

use crate::error::{WithingsError, WithingsResult};
use crate::types::ApiEnvelope;

const DEFAULT_AUTH_URL: &str = "https://account.withings.com/oauth2_user/authorize2";
const DEFAULT_TOKEN_URL: &str = "https://wbsapi.withings.net/v2/oauth2";

/// Body of a successful Withings token response.
///
/// Wrapped inside `ApiEnvelope` on the wire.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub userid: String,
    pub access_token: String,
    pub refresh_token: String,
    /// Lifetime of `access_token` in seconds. Withings docs: 10800 (3h).
    pub expires_in: i64,
    pub scope: String,
    pub token_type: String,
}

/// OAuth 2.0 client for Withings.
pub struct WithingsOauth {
    client_id: ClientId,
    client_secret: ClientSecret,
    redirect_uri: RedirectUrl,
    auth_url: AuthUrl,
    token_url: TokenUrl,
    http: reqwest::Client,
}

impl WithingsOauth {
    /// Construct using Withings production endpoints.
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> WithingsResult<Self> {
        Self::with_urls(
            client_id,
            client_secret,
            redirect_uri,
            DEFAULT_AUTH_URL.to_string(),
            DEFAULT_TOKEN_URL.to_string(),
        )
    }

    /// Construct with explicit URLs (used by tests with mock servers).
    pub fn with_urls(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        auth_url: String,
        token_url: String,
    ) -> WithingsResult<Self> {
        Ok(Self {
            client_id: ClientId::new(client_id),
            client_secret: ClientSecret::new(client_secret),
            redirect_uri: RedirectUrl::new(redirect_uri)
                .map_err(|e| WithingsError::Config(format!("invalid redirect_uri: {e}")))?,
            auth_url: AuthUrl::new(auth_url)
                .map_err(|e| WithingsError::Config(format!("invalid auth_url: {e}")))?,
            token_url: TokenUrl::new(token_url)
                .map_err(|e| WithingsError::Config(format!("invalid token_url: {e}")))?,
            http: reqwest::Client::builder()
                .build()
                .map_err(WithingsError::Http)?,
        })
    }

    /// Build the URL to send the user's browser to for consent.
    /// Returns the URL and the random CSRF state — caller must store the
    /// state and verify it matches the `state` param when the redirect arrives.
    pub fn authorization_url(&self, scopes: &[&str]) -> (url::Url, CsrfToken) {
        let csrf = CsrfToken::new_random();
        let mut url =
            url::Url::parse(self.auth_url.as_str()).expect("AUTH_URL validated at construction");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", self.client_id.as_str())
            .append_pair("scope", &scopes.join(","))
            .append_pair("redirect_uri", self.redirect_uri.as_str())
            .append_pair("state", csrf.secret());
        (url, csrf)
    }

    /// Exchange an authorization `code` for tokens.
    pub async fn exchange_code(&self, code: &str) -> WithingsResult<TokenResponse> {
        let envelope: ApiEnvelope<TokenResponse> = self
            .http
            .post(self.token_url.as_str())
            .form(&[
                ("action", "requesttoken"),
                ("grant_type", "authorization_code"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.secret()),
                ("code", code),
                ("redirect_uri", self.redirect_uri.as_str()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        unwrap_envelope(envelope)
    }

    /// Use a refresh token to obtain a fresh access token.
    pub async fn refresh(&self, refresh_token: &str) -> WithingsResult<TokenResponse> {
        let envelope: ApiEnvelope<TokenResponse> = self
            .http
            .post(self.token_url.as_str())
            .form(&[
                ("action", "requesttoken"),
                ("grant_type", "refresh_token"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.secret()),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        unwrap_envelope(envelope)
    }
}

fn unwrap_envelope<T>(envelope: ApiEnvelope<T>) -> WithingsResult<T> {
    if envelope.is_success() {
        envelope
            .body
            .ok_or_else(|| WithingsError::UnexpectedResponse("status=0 but body is null".into()))
    } else {
        Err(WithingsError::Api {
            status: envelope.status,
            message: envelope.error.unwrap_or_default(),
        })
    }
}
