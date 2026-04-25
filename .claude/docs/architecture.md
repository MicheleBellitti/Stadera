# Stadera — Architecture & Roadmap

Living document. Single source of truth for cross-milestone decisions that are **not** in the code yet or that would require digging through conversations to reconstruct. Code-level conventions live in `.claude/CLAUDE.md`.

## Project

**Stadera** — Personal weight-tracking and nutrition-coaching app backed by Withings smart scales. Named after the *stadera romana*, the ancient Roman steelyard balance.

**Primary goal**: controlled 10 kg weight loss (83 → 73 kg, height 1.73 m) with personalized kcal/protein plan + daily/weekly notifications.
**Secondary goal**: learn cloud-native Rust properly while building something useful.

**Owner** — Michele Bellitti. Single-user app. Repo is private.

## Repositories

- `stadera` (this repo) — Rust backend, monorepo via Cargo workspace.
- `stadera-web` (not yet created) — Next.js frontend. To be scaffolded in M5 by Claude; architectural choices consulted with owner.

## Workspace layout

```
stadera/
├── crates/
│   ├── api/           # axum HTTP server — scaffolded, built out in M5
│   ├── domain/        # pure business logic — M2 (in review as PR #2)
│   ├── storage/       # Postgres repos + sqlx — planned M3
│   ├── withings/      # Withings OAuth2 + API client — planned M4
│   ├── notifications/ # Pushover + Resend — planned M6
│   └── jobs/          # cron binary (daily sync, weekly digest) — planned M4/M6
├── .github/workflows/
├── .claude/           # Claude memory + docs (this file lives here)
├── Cargo.toml         # workspace manifest
├── rust-toolchain.toml
├── release-please-config.json
└── …
```

## Stack

| Layer | Choice | Why |
|---|---|---|
| Language | Rust edition 2024, toolchain 1.85+ | Statically-typed domain, async mature, learning target |
| Async runtime | Tokio | De-facto standard |
| HTTP | `axum` | Tower ecosystem, ergonomic, well-integrated |
| DB | Postgres (Neon serverless) | Free tier, scale-to-zero, SQL familiar |
| DB client | `sqlx` | Compile-time SQL check, async, migrations built-in |
| OpenAPI | `utoipa` (code-first) + Swagger UI | Schema from types, zero duplication |
| Errors | `thiserror` (libs) + `anyhow` (bins) | Idiomatic split |
| Serde | `serde` + `serde_json` | De-facto |
| Time | `chrono` (with `serde`) | Richer API than `time` for this use case |
| Tracing | `tracing` + `tracing-subscriber` | Structured logs, span support |
| Tests | built-in + `proptest` | Property testing for domain invariants |

## Deployment target (M7)

| Concern | Choice |
|---|---|
| Backend runtime | Cloud Run (scale-to-zero) |
| Cron | Cloud Run Jobs + Cloud Scheduler |
| Database | Neon Postgres (serverless, free tier) |
| Secrets | GCP Secret Manager |
| IaC | Terraform |
| Frontend | Vercel |
| Logging | Native GCP (no Sentry for now) |
| Observability | Cloud Logging / Cloud Trace (TBD) |
| Environments | Single env — no dev/prod split. Feature branches test locally. |

## Roadmap

| ID | Name | Status | Notes |
|---|---|---|---|
| M1 | Foundations | ✅ Done | Workspace, CI, release-please, conventional commits |
| M2 | Domain core | ✅ Done | Merged via PR #2 (squashed into `1507c18`). Domain blockers resolved. Follow-ups tracked in `reviews/pr-2.md` |
| M3 | Storage | ✅ Done | Both PRs merged: scaffold (#3) and repositories+tests (#5). Closed `1507c18..4730651` |
| M4 | Withings integration | ⏳ In progress | PR A `feat/withings-scaffold` merged (#6): OAuth2 + API client + crypto + pair binary. PR B `feat/sync-job` open: `crates/jobs` cron binary that syncs measurements end-to-end |
| M5 | API + Frontend | 📋 Planned | axum endpoints (`/today`, `/trend`, `/history`), OAuth Google auth middleware, utoipa/Swagger. `stadera-web` repo scaffolded |
| M6 | Notifications | 📋 Planned | Pushover daily job, Resend weekly digest (Apple-weekly-summary style) |
| M7 | Cloud deploy | 📋 Planned | Terraform, GitHub Actions deploy, Dockerfile multi-stage, Vercel for frontend |

## Key architectural decisions

### Single-tenant but multi-tenant-ready schema

- Auth: real OAuth Google with a single exposed user. No API keys.
- Every DB table has `user_id` from day 1.
- Rationale: avoid a schema migration if/when the app is ever opened to others. Cost of `user_id` columns is negligible; cost of retrofitting them later is high.

### Domain crate isolation (M2)

- Pure business logic, zero I/O.
- No `Utc::now()` / `SystemTime::now()` — time is injected as parameter for testability.
- All domain values are newtypes with validating constructors. No raw `f64`/`i64` at the domain boundary.
- Tests in `tests/` dir (integration-test style) instead of inline `#[cfg(test)]` — forces testing through the public API.

### Storage layer (M3) — resolved

- **Repository pattern**: concrete structs (`PgMeasurementRepository`, …), no traits for now. Integration tests against a real DB. Traits only when a mock actually becomes necessary.
- **Migrations lifecycle**: `sqlx migrate` CLI for local dev + `sqlx::migrate!().run(&pool)` inside the binary on startup (Cloud Run self-migrates on cold start). Same `migrations/` directory feeds both.
- **Connection pool**: `max_connections = 10`, `connect_timeout = 5s`. Tuned finer in M7 when observing actual load on Cloud Run + Neon pooler.
- **Dev env**: `compose.yaml` with Postgres 16, root-level `Makefile` for `db-up` / `db-down` / `db-migrate` / `db-reset` / `db-psql`.
- **Schema**: UUID v7 for PKs (time-ordered, cloud-native), `timestamptz` for all temporal columns, `varchar + CHECK` for enums (easier to evolve than native Postgres enum).
- **Tables in M3**: `users`, `user_profiles`, `measurements`, `withings_credentials`. The last one is pre-provisioned even though its usage lands in M4 — avoids an extra migration PR.
- **Domain ↔ SQL mapping**: repository performs `Weight::try_from(f64)?` on read; returns `StorageError::Corruption` if the DB produced an invalid value (should not happen, but explicit).
- **`Measurement::source`** (`Withings` | `Manual`): extended in the domain now to avoid a second-wave domain bump in M4.
- **Withings token encryption**: column `BYTEA` in M3; encryption logic lands in M4 together with the OAuth flow (key from Secret Manager).
- **Integration tests**: `sqlx::test` macro (lighter than testcontainers). Requires a reachable `DATABASE_URL` in CI — add a `postgres` service to `ci.yml`.

### Notifications (M6 decisions pending)

- Pushover daily: what time? Morning (6-8) before the workout, or evening with yesterday's summary?
- Weekly digest email: Sunday evening or Monday morning?
- Email template: HTML with MJML, or plain-ish styled?

### Cloud (M7 — partially resolved)

- **GCP project**: `stadera-prod` (single project, single env — matches the single-env decision).
- **Neon layout**: 1 project `stadera`, default `main` branch (Neon's copy-on-write branches), database `stadera`, dedicated role `stadera_app` (least privilege, not the `neondb_owner`). Neon branching available for preview envs in the future, out-of-scope for now.
- **Custom domain** vs `.run.app`: still open. Decide in M7.
- **Preview envs**: Vercel previews on frontend PRs; backend stays single-env (feature branches test locally against Docker).

## Conventions (quick reference)

Full details in `.claude/CLAUDE.md`. Key points for fast recall:

- **Trunk-based**: `main` always deployable. Short-lived feature branches.
- **Branch naming**: `<type>/<short-desc>` — e.g. `feat/storage-schema`, `fix/serde-validation`, `chore/pr-template`.
- **Conventional Commits**: enforced. Squash merge: 1 PR = 1 commit on `main`.
- **release-please**: on push to `main`, automatic changelog + version bump + GitHub Release.
- **CI gate**: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --all` must all pass.
- **No `unwrap()` / `panic!()` / `expect()` in non-test code.**
- **Review flow**: Michele implements backend; Claude reviews. Claude implements frontend (M5+), consults on architecture.

## Mentoring contract

This is the working agreement between Michele and Claude (mirrored in `.claude/CLAUDE.md`):

1. Claude is mentor + reviewer for backend, **not** code generator.
2. Every non-trivial task → multi-step plan validated by owner before execution.
3. Clarifying questions over guesswork.
4. Exception: docs / tests / CI / markdown / config — Claude can proceed directly.
5. For frontend Next.js (M5+): Claude implements, consults on architecture.
6. Italian in conversation when asked in Italian; English in code, commits, docs, PR templates (repo-level lingua franca).

## Living memory

Cross-milestone facts worth remembering:

- Owner's starting metrics (as of project start): weight 83 kg, height 1.73 m, target 73 kg.
- Protein target convention: typically 1.6–2.0 g / kg bodyweight during cut.
- Deficit convention: 500 kcal/day → ~0.5 kg/week (sustainable floor).
- Medical safety floor (adult male): 1500 kcal/day minimum. For adult female: 1200.
- Withings API is the only data ingestion source. No manual weight entry endpoint planned.

---

Last updated: 2026-04-24. Review and prune when a milestone closes or a decision becomes code.
