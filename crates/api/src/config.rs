//! Application configuration loaded from environment variables.

use std::net::SocketAddr;

use anyhow::{Context, Result};

/// Resolved runtime configuration for the API.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
}

impl Config {
    /// Read config from env. `PORT` defaults to 3000 (Cloud Run sets it
    /// automatically in production).
    pub fn from_env() -> Result<Self> {
        let database_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL env var is required")?;

        let port: u16 = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .context("PORT must be a valid u16 between 1 and 65535")?;

        // 0.0.0.0 so Docker / Cloud Run can route external traffic.
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], port));

        Ok(Self {
            database_url,
            bind_addr,
        })
    }
}
