//! Stadera cron / job entry point.
//!
//! Single binary with subcommands so the same image can run different jobs
//! (Cloud Run Jobs invokes one subcommand per scheduler trigger).
//!
//! Available subcommands:
//! - `sync` — pull recent measurements from Withings and persist them.
//! - (future) `digest` — weekly Resend email digest (M6).

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod sync;

#[derive(Parser)]
#[command(name = "stadera-jobs", version, about = "Stadera cron jobs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Pull recent measurements from Withings into the database.
    Sync {
        /// Email of the user whose measurements to sync.
        #[arg(long)]
        user_email: String,
        /// Sync window in days (defaults to 7).
        #[arg(long, default_value_t = 7)]
        window_days: i64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Auto-load `.env` for local runs; no-op in production where env vars
    // come from Cloud Run / Secret Manager.
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Sync {
            user_email,
            window_days,
        } => sync::run(&user_email, window_days).await,
    }
}
