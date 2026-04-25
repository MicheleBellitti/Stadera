// One-shot CLI tool for first-time OAuth pairing with Withings.

use std::io::{BufRead, BufReader, Write};

use anyhow::{Context, bail};
use chrono::{Duration, Utc};
use clap::Parser;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

use stadera_storage::{StorageContext, WithingsCredentials};
use stadera_withings::crypto;
use stadera_withings::oauth::WithingsOauth;

/// One-shot Withings OAuth pairing tool for Stadera.
#[derive(Parser)]
#[command(name = "stadera-pair")]
struct Cli {
    /// Email of the user to pair.
    #[arg(long)]
    user_email: String,

    /// Display name for the user (defaults to email if omitted).
    #[arg(long)]
    user_name: Option<String>,
}

const REDIRECT_URI: &str = "http://localhost:7878/callback";
const SCOPES: &[&str] = &["user.metrics"];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Auto-load `.env` from cwd (or any ancestor). No-op in production where the
    // file is absent — env vars are injected by the runtime (Cloud Run secrets).
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // --- 1. Read env ---
    let client_id =
        std::env::var("WITHINGS_CLIENT_ID").context("WITHINGS_CLIENT_ID env var is required")?;
    let client_secret = std::env::var("WITHINGS_CLIENT_SECRET")
        .context("WITHINGS_CLIENT_SECRET env var is required")?;
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL env var is required")?;

    // Build cipher early so we fail fast if the key is missing/invalid.
    let cipher = crypto::cipher_from_env()
        .context("failed to load encryption key from WITHINGS_TOKEN_KEY")?;

    // --- 2. Build authorization URL ---
    let oauth = WithingsOauth::new(client_id, client_secret, REDIRECT_URI.to_string())
        .context("failed to build WithingsOauth")?;

    let (auth_url, csrf_state) = oauth.authorization_url(SCOPES);

    // --- 3. Print URL ---
    println!("\nOpen this URL in your browser to authorize Stadera:\n");
    println!("  {auth_url}\n");
    println!("Waiting for callback on {REDIRECT_URI} …\n");

    // --- 4. Spawn TCP listener ---
    let listener = TcpListener::bind("127.0.0.1:7878")
        .await
        .context("failed to bind to 127.0.0.1:7878")?;

    // --- 5. Accept ONE connection, parse the callback ---
    let (stream, _addr) = listener
        .accept()
        .await
        .context("failed to accept connection")?;

    // Read the first HTTP request line from the TCP stream.
    let std_stream = stream
        .into_std()
        .context("failed to convert tokio stream to std")?;
    std_stream
        .set_nonblocking(false)
        .context("failed to set blocking mode")?;
    let mut reader = BufReader::new(&std_stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .context("failed to read HTTP request line")?;

    // Parse "GET /callback?code=...&state=... HTTP/1.1"
    let path = request_line
        .split_whitespace()
        .nth(1)
        .context("malformed HTTP request line")?
        .to_string();

    // Drain remaining HTTP headers so the browser doesn't see a connection reset.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
    }

    let full_url = Url::parse(&format!("http://localhost:7878{path}"))
        .context("failed to parse callback URL")?;

    let params: std::collections::HashMap<String, String> = full_url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let code = params
        .get("code")
        .context("callback is missing `code` parameter")?;
    let state = params
        .get("state")
        .context("callback is missing `state` parameter")?;

    // --- 6. Validate CSRF state ---
    if state != csrf_state.secret() {
        // Write error page before bailing.
        let error_html = "HTTP/1.1 400 Bad Request\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            Connection: close\r\n\r\n\
            <h1>CSRF state mismatch</h1>\
            <p>The state parameter does not match. Please try again.</p>";
        write_response(&std_stream, error_html);
        bail!(
            "CSRF state mismatch: expected {}, got {state}",
            csrf_state.secret()
        );
    }

    info!("received valid callback, exchanging code for tokens …");

    // --- 7. Exchange code for tokens ---
    let tokens = oauth
        .exchange_code(code)
        .await
        .context("failed to exchange authorization code for tokens")?;

    info!(userid = %tokens.userid, scope = %tokens.scope, "token exchange successful");

    // --- 8. Connect Postgres ---
    let pool = PgPool::connect(&database_url)
        .await
        .context("failed to connect to Postgres")?;

    let storage = StorageContext::new(pool);

    // --- 9. Look up user by email (or create if missing) ---
    let user = match storage.users().get_by_email(&cli.user_email).await? {
        Some(u) => {
            info!(user_id = %u.id, "found existing user");
            u
        }
        None => {
            warn!(email = %cli.user_email, "user not found — creating");
            let name = cli.user_name.as_deref().unwrap_or(&cli.user_email);
            let id = storage.users().create(&cli.user_email, name).await?;
            storage
                .users()
                .get_by_id(id)
                .await?
                .context("failed to read back newly created user")?
        }
    };

    // --- 10. Encrypt tokens ---
    let access_token_enc = crypto::encrypt(&cipher, tokens.access_token.as_bytes())
        .context("failed to encrypt access token")?;
    let refresh_token_enc = crypto::encrypt(&cipher, tokens.refresh_token.as_bytes())
        .context("failed to encrypt refresh token")?;

    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in);

    // --- 11. Upsert credentials ---
    let creds = WithingsCredentials {
        user_id: user.id,
        access_token_enc,
        refresh_token_enc,
        expires_at,
        scope: tokens.scope,
    };

    storage
        .withings_credentials()
        .upsert(&creds)
        .await
        .context("failed to upsert Withings credentials")?;

    info!(user_id = %user.id, "credentials stored successfully");

    // --- 12. Write success HTML page, flush, and close ---
    let success_html = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Connection: close\r\n\r\n\
        <!DOCTYPE html>\
        <html><body>\
        <h1>Stadera paired successfully!</h1>\
        <p>You can close this tab and return to the terminal.</p>\
        </body></html>";
    let mut writable = std_stream;
    writable.write_all(success_html.as_bytes())?;
    writable.flush()?;
    drop(writable);

    // --- 13. Exit cleanly ---
    println!(
        "Pairing complete for {}. You can close this terminal.",
        cli.user_email
    );

    Ok(())
}

/// Best-effort write to the TCP stream; used for error responses before bail.
fn write_response(stream: &std::net::TcpStream, response: &str) {
    use std::io::Write;
    if let Err(e) = stream.try_clone().and_then(|mut s| {
        s.write_all(response.as_bytes())?;
        s.flush()
    }) {
        warn!("failed to write HTTP response to browser: {e}");
    }
}
