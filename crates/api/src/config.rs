//! Application configuration loaded from environment variables.

use std::net::SocketAddr;

use anyhow::{Context, Result};

/// Resolved runtime configuration for the API.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub frontend_origin: String,
    /// `true` in production (HTTPS) — sets `Secure` flag on cookies.
    /// `false` in dev (HTTP localhost).
    pub cookie_secure: bool,
    /// Optional explicit `Domain` attribute on cookies.
    ///
    /// - `None` (env var unset) → host-only cookies. The browser only
    ///   sends them back to the exact host that issued them. This is
    ///   the right default for dev (FE+BE on `localhost`, port-agnostic
    ///   so cookies are shared) and for any single-host production.
    /// - `Some(".stadera.org")` → cookie shared across every subdomain.
    ///   Required when FE and BE are on different subdomains of a
    ///   common parent (e.g. `app.stadera.org` + `api.stadera.org`).
    ///   Browsers reject `Domain` values not matching the host that's
    ///   setting the cookie, so the value MUST be a parent of the
    ///   backend's hostname.
    pub cookie_domain: Option<String>,
    pub google: GoogleConfig,
}

/// Google OAuth 2.0 client configuration.
#[derive(Debug, Clone)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Must match the redirect URI registered in the GCP OAuth consent screen.
    pub redirect_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL env var is required")?;

        let port: u16 = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .context("PORT must be a valid u16 between 1 and 65535")?;
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], port));

        let frontend_origin = std::env::var("FRONTEND_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let cookie_secure = std::env::var("COOKIE_SECURE")
            .map(|s| s.eq_ignore_ascii_case("true") || s == "1")
            .unwrap_or(false);

        // Treat empty string the same as unset — useful when the env
        // var is plumbed through CI / deploy infra that always
        // expands `${{ vars.X }}` even when X has no value.
        let cookie_domain = std::env::var("COOKIE_DOMAIN")
            .ok()
            .filter(|s| !s.is_empty());

        let google = GoogleConfig {
            client_id: std::env::var("GOOGLE_CLIENT_ID")
                .context("GOOGLE_CLIENT_ID env var is required")?,
            client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
                .context("GOOGLE_CLIENT_SECRET env var is required")?,
            redirect_url: std::env::var("GOOGLE_REDIRECT_URL")
                .unwrap_or_else(|_| "http://localhost:3000/auth/google/callback".to_string()),
        };

        Ok(Self {
            database_url,
            bind_addr,
            frontend_origin,
            cookie_secure,
            cookie_domain,
            google,
        })
    }
}
