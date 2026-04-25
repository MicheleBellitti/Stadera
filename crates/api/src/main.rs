use anyhow::{Context, Result};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use stadera_api::{AppState, config::Config, router};

#[tokio::main]
async fn main() -> Result<()> {
    // Auto-load `.env` locally; no-op in production where env vars come from
    // Cloud Run / Secret Manager injection.
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,stadera_api=debug,tower_http=debug")),
        )
        .init();

    let config = Config::from_env().context("invalid configuration")?;

    let pool = PgPool::connect(&config.database_url)
        .await
        .context("failed to connect to Postgres")?;

    let state = AppState::new(pool);
    let app = router(state);

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind to {}", config.bind_addr))?;

    tracing::info!(addr = %config.bind_addr, "stadera-api listening");

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
