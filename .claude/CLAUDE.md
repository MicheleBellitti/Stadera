# CLAUDE.md

Guidance for Claude Code when working on the Stadera repository.

## Project

**Stadera** — Personal weight tracking & nutrition coaching API, integrated with 
Withings smart scales. Named after the *stadera romana*, the ancient steelyard balance.

Backend-only repo. Frontend lives in a separate `stadera-web` repo (Next.js).

## Working agreement

The repo owner (Michele) is implementing the backend himself with Claude acting 
as **mentor and reviewer**, not code generator. When asked to help with backend:

1. Propose a multi-step plan and wait for validation before executing
2. Ask clarifying questions when anything is ambiguous
3. Do not write production Rust code unsolicited — guide, review, suggest patterns
4. Exception: when explicitly asked to implement, or when writing tests/docs/CI, 
   proceed directly
5. The frontend (separate repo) is fully Claude-implemented when that repo exists

Respond in Italian when addressed in Italian, English otherwise.

## Architecture

Cargo workspace, crates under `crates/`:

| Crate           | Responsibility                                        | Status |
| --------------- | ----------------------------------------------------- | ------ |
| `domain`        | Pure business logic. Types, validation, calculations. No I/O. | ✅ M2 |
| `storage`       | Postgres repository pattern, sqlx migrations.         | planned M3 |
| `withings`      | Withings OAuth2 + Health Mate API client.             | planned M4 |
| `notifications` | Pushover + Resend (email) clients.                    | planned M6 |
| `api`           | axum HTTP server binary. utoipa for OpenAPI.          | scaffolded, expanded M5 |
| `jobs`          | Cron binary for daily sync + weekly digest.           | planned M4/M6 |

**Data flow**: Withings → `jobs` (sync) → Postgres → `api` → `stadera-web` frontend.  
**Auth model**: Single-tenant with real OAuth Google. Schema is multi-user-ready 
(every table has `user_id`), but only one user is exposed. No API keys.

**Target deployment**: GCP — Cloud Run (api), Cloud Run Jobs + Scheduler (jobs), 
Neon Postgres, Secret Manager. Terraform for IaC. Single env (no dev/prod split).

## Rust conventions (strict)

- Toolchain pinned in `rust-toolchain.toml`. Never bypass it.
- Edition 2024 across the workspace via `edition.workspace = true`.
- Centralize shared deps in `[workspace.dependencies]` — crates inherit with 
  `dep.workspace = true`.
- **No `unwrap()` / `panic!()` / `expect()` in non-test code.** Use `Result` 
  with `thiserror` for library crates, `anyhow` for binaries.
- **No raw `f64`/`i64`/etc. for domain values.** Use newtypes with validating 
  constructors (e.g. `Weight::new(f64) -> Result<Self, DomainError>`).
- **Do not derive `Eq`/`Hash` on types containing `f64`** — only `PartialEq`/`PartialOrd`.
- **Never call `Utc::now()` / `SystemTime::now()` inside `domain/`**. Inject 
  time as a parameter for testability.
- Re-export public types at crate root (`lib.rs`) for ergonomic imports.
- Prefer `TryFrom` for fallible conversions, `From` for infallible.
- Async runtime is Tokio. Async-in-trait via `async_trait` crate when needed 
  (until stabilized enough to drop it).

## Commands

Before any commit or PR, these must pass locally:

```bash