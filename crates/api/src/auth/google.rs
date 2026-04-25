//! Google OAuth 2.0 flow.
//!
//! Uses the `oauth2` crate end-to-end (Google is RFC 6749-compliant, so
//! no envelope-bypass needed — unlike Withings). For each operation we
//! rebuild the `BasicClient` to avoid pinning the typestate generics into
//! a stored field; the cost is a few `String` clones, irrelevant compared
//! to the network round-trip.

use anyhow::Context;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;

use crate::config::GoogleConfig;
use crate::error::AppError;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

pub struct GoogleClient {
    config: GoogleConfig,
    http: reqwest::Client,
}

/// Profile info returned by Google's userinfo endpoint.
#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub picture: Option<String>,
}

impl GoogleClient {
    pub fn new(config: GoogleConfig) -> Result<Self, AppError> {
        let http = reqwest::Client::builder()
            .build()
            .context("building reqwest client")
            .map_err(AppError::Internal)?;
        Ok(Self { config, http })
    }

    /// Build the URL to redirect the user's browser to. The returned
    /// `CsrfToken` must be persisted (e.g. in a cookie) and verified at
    /// the callback.
    pub fn authorize_url(&self) -> Result<(url::Url, CsrfToken), AppError> {
        let client = self.build_client()?;
        Ok(client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url())
    }

    /// Exchange an authorization code for tokens, then fetch the user's
    /// profile from Google. Returns only the profile — we do not persist
    /// Google's access/refresh tokens (Stadera only needs identity).
    pub async fn exchange_code(&self, code: String) -> Result<GoogleUserInfo, AppError> {
        let client = self.build_client()?;
        let token_response = client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(&self.http)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("token exchange: {e}")))?;

        let access_token = oauth2::TokenResponse::access_token(&token_response)
            .secret()
            .to_string();

        let userinfo: GoogleUserInfo = self
            .http
            .get(GOOGLE_USERINFO_URL)
            .bearer_auth(&access_token)
            .send()
            .await
            .context("userinfo request")
            .map_err(AppError::Internal)?
            .error_for_status()
            .context("userinfo http status")
            .map_err(AppError::Internal)?
            .json()
            .await
            .context("decoding userinfo body")
            .map_err(AppError::Internal)?;

        Ok(userinfo)
    }

    fn build_client(
        &self,
    ) -> Result<
        BasicClient<
            oauth2::EndpointSet,
            oauth2::EndpointNotSet,
            oauth2::EndpointNotSet,
            oauth2::EndpointNotSet,
            oauth2::EndpointSet,
        >,
        AppError,
    > {
        let auth_url = AuthUrl::new(GOOGLE_AUTH_URL.to_string())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("static AUTH_URL invalid: {e}")))?;
        let token_url = TokenUrl::new(GOOGLE_TOKEN_URL.to_string())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("static TOKEN_URL invalid: {e}")))?;
        let redirect = RedirectUrl::new(self.config.redirect_url.clone())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("redirect url invalid: {e}")))?;

        Ok(
            BasicClient::new(ClientId::new(self.config.client_id.clone()))
                .set_client_secret(ClientSecret::new(self.config.client_secret.clone()))
                .set_auth_uri(auth_url)
                .set_token_uri(token_url)
                .set_redirect_uri(redirect),
        )
    }
}
