# Stadera

Personal weight tracking & nutrition coaching API, powered by Withings
smart scales.

Named after the *stadera romana*, the ancient steelyard balance.

## Status

M5 complete (API + frontend). M7-step1 (backend Cloud Run deploy) in
progress. M6 (notifications), M7-step2/3 (sync job + Terraform IaC) and
optional backlog issues are next.

## Architecture

- Rust backend (axum + sqlx + Tokio), workspace under `crates/`
  - `domain` — pure business logic, no I/O
  - `storage` — Postgres repository pattern, sqlx migrations
  - `withings` — Withings OAuth2 + Health Mate API client
  - `api` — axum HTTP server (binary)
  - `jobs` — cron binary for daily sync
- Postgres (Neon, serverless)
- Google Cloud Run for both `api` (service) and `jobs` (job, M7-step2)
- Next.js frontend in [stadera-web](https://github.com/MicheleBellitti/stadera-web)

## Local development

Requires:

- Rust (toolchain pinned in `rust-toolchain.toml`)
- Docker for Postgres
- `sqlx-cli` for migrations: `cargo install sqlx-cli --no-default-features -F postgres,rustls`

Common targets (see `make help`):

```sh
make db-up         # start local Postgres on :5432
make db-migrate    # apply migrations
make pair USER_EMAIL=...   # one-shot Withings OAuth pairing
make sync USER_EMAIL=...   # one-shot Withings sync
make check         # fmt + clippy + test (CI gate)
```

Env vars are read from `.env` (auto-loaded via `dotenvy`). No
`.env.example` — required vars listed in `crates/api/src/config.rs`.

## Deploy

Cloud Run, scale-to-zero, deployed on every push to `main` via
`.github/workflows/deploy.yml`. Auth is via Workload Identity Federation
— no JSON service-account keys.

### Container layout

`Dockerfile` is a 4-stage build using `cargo-chef` so dependency
compilation is cached as a separate Docker layer. Without this, every
source change re-builds axum/sqlx/reqwest/oauth2 from scratch (~5 min).
With it, source-only changes finish in ~30 s.

```
chef    → install cargo-chef
planner → cargo chef prepare → recipe.json (dependency manifest)
builder → cargo chef cook (builds all deps), then cargo build (binaries)
runtime → distroless cc-debian12, ~25 MB base, both binaries copied in
```

Final image is ~120 MB. CMD defaults to `stadera-api`; the Cloud Run
Job for the daily sync (M7-step2) overrides CMD to `stadera-jobs sync`.

```sh
# Local smoke test
make docker-build
make docker-run        # binds :8080, reads .env
```

### Required GitHub configuration

The deploy workflow expects a GitHub Environment named `prod` with
the variables and secrets documented in the header of
[`.github/workflows/deploy.yml`](.github/workflows/deploy.yml). Names:

- **Variables**: `GCP_PROJECT`, `GCP_REGION`, `ARTIFACT_REGISTRY_REPO`,
  `CLOUD_RUN_SERVICE`, `CLOUD_RUN_SYNC_JOB`, `SYNC_USER_EMAIL`,
  `FRONTEND_ORIGIN`, `COOKIE_DOMAIN`, `GOOGLE_CLIENT_ID`,
  `GOOGLE_REDIRECT_URL`, `WITHINGS_CLIENT_ID`
- **Secrets**: `WIF_PROVIDER`, `WIF_SERVICE_ACCOUNT`, `DATABASE_URL`,
  `GOOGLE_CLIENT_SECRET`, `WITHINGS_CLIENT_SECRET`, `WITHINGS_TOKEN_KEY`

### One-time GCP setup

You'll need:

- A GCP project with Artifact Registry, Cloud Run, Cloud Scheduler,
  IAM Credentials, and STS APIs enabled
- A deployer service account with project-level grants:
  `roles/run.admin`, `roles/artifactregistry.writer`,
  `roles/iam.serviceAccountUser`
- A Workload Identity Pool + OIDC provider scoped to your fork
  via `attribute.repository=='<owner>/<repo>'`
- The provider bound to the deployer SA via
  `roles/iam.workloadIdentityUser` on a `principalSet://` member
- A Cloud Scheduler job triggering the Cloud Run Job daily, signed
  by a separate `*-invoker` SA with `run.invoker` on the job

The exact `gcloud` sequence + import-into-Terraform are kept in a
private operational doc (the public repo deliberately doesn't ship
copy-paste commands tied to a specific project ID). The shape is in
[`terraform/`](./terraform) once that PR lands.

### Custom domain (recommended)

Map `api.<your-domain>` and `app.<your-domain>` to the Cloud Run
services. With both subdomains under the same parent and
`COOKIE_DOMAIN=.<your-domain>` on the backend, sessions work
across FE/BE without CORS gymnastics. DNS-only (no proxy) is
required by Cloud Run's TLS provisioning.

Without a custom domain, FE and BE end up on different `*.run.app`
hosts and browser cookie isolation breaks the auth flow. Plan
accordingly.

## License

MIT
