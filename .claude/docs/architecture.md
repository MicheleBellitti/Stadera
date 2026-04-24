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
| M2 | Domain core | ⏳ In review | PR #2 open. 3 blockers to fix — see `reviews/pr-2.md` |
| M3 | Storage | ⏸ Next | Postgres schema, sqlx migrations, repository pattern, Docker compose for local dev |
| M4 | Withings integration | 📋 Planned | OAuth2 flow, token refresh, API client, sync binary. Multi-tenant DB schema ready |
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

### Storage layer (M3 decisions pending)

Open questions to resolve when starting M3:
- Repository pattern via traits (for testability) vs. direct `sqlx` calls in services?
- Migration strategy: `sqlx migrate` CLI vs. a lib-level migration runner at startup?
- Connection pooling config — `sqlx::PgPool` defaults or custom sizing for Cloud Run (scale-to-zero → cold starts)?
- Docker compose for local Postgres 16? (probably yes)

### Notifications (M6 decisions pending)

- Pushover daily: what time? Morning (6-8) before the workout, or evening with yesterday's summary?
- Weekly digest email: Sunday evening or Monday morning?
- Email template: HTML with MJML, or plain-ish styled?

### Cloud (M7 decisions pending)

- Single GCP project or separate for Stadera?
- Custom domain from day 1 or `.run.app` URL?
- Staging/preview envs on Vercel previews only, backend always single-env?

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
