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

### One-time GCP setup

These run once per project. The frontend setup
([stadera-web README](https://github.com/MicheleBellitti/stadera-web#deploy))
already creates the Artifact Registry repo and the Workload Identity
Pool — we reuse them here, only the service account and the
provider-per-repo are new.

For a step-by-step explanation of each `gcloud` command, see
[`.claude/docs/deploy-gcp-walkthrough.md`](.claude/docs/deploy-gcp-walkthrough.md).

```sh
PROJECT=...                      # GCP project ID
REGION=europe-west1
REPO=stadera                     # Artifact Registry repo (shared with web)
SERVICE=stadera-api
SA=stadera-api-deployer
SA_EMAIL=$SA@$PROJECT.iam.gserviceaccount.com

POOL=github-pool                 # reuse from web setup
PROVIDER=stadera-api-provider    # new: scoped to this repo
GITHUB_REPO=MicheleBellitti/Stadera

# 1. Service account for deploys
gcloud iam service-accounts create $SA --project=$PROJECT

gcloud projects add-iam-policy-binding $PROJECT \
    --member="serviceAccount:$SA_EMAIL" --role=roles/run.admin
gcloud projects add-iam-policy-binding $PROJECT \
    --member="serviceAccount:$SA_EMAIL" --role=roles/artifactregistry.writer
gcloud projects add-iam-policy-binding $PROJECT \
    --member="serviceAccount:$SA_EMAIL" --role=roles/iam.serviceAccountUser

# 2. New OIDC provider in the existing pool, scoped to THIS repo
gcloud iam workload-identity-pools providers create-oidc $PROVIDER \
    --location=global --workload-identity-pool=$POOL --project=$PROJECT \
    --display-name="GitHub Actions (stadera backend)" \
    --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
    --attribute-condition="assertion.repository=='$GITHUB_REPO'" \
    --issuer-uri="https://token.actions.githubusercontent.com"

# 3. Bind GitHub repo identity → service account
POOL_ID=$(gcloud iam workload-identity-pools describe $POOL \
    --location=global --project=$PROJECT --format='value(name)')

gcloud iam service-accounts add-iam-policy-binding $SA_EMAIL \
    --project=$PROJECT --role=roles/iam.workloadIdentityUser \
    --member="principalSet://iam.googleapis.com/$POOL_ID/attribute.repository/$GITHUB_REPO"

# 4. Print the values to paste into GitHub secrets
echo "WIF_PROVIDER       = $POOL_ID/providers/$PROVIDER"
echo "WIF_SERVICE_ACCOUNT = $SA_EMAIL"
```

### GitHub configuration

In *Settings → Secrets and variables → Actions*:

**Variables** (non-secret operational config):

- `GCP_PROJECT` — GCP project ID
- `GCP_REGION` — e.g. `europe-west1`
- `ARTIFACT_REGISTRY_REPO` — `stadera`
- `CLOUD_RUN_SERVICE` — `stadera-api`
- `FRONTEND_ORIGIN` — public URL of the deployed frontend Cloud Run service
- `GOOGLE_CLIENT_ID` — Google OAuth client ID (public)
- `GOOGLE_REDIRECT_URL` — `${BACKEND_URL}/auth/google/callback`

**Secrets** (real credentials):

- `WIF_PROVIDER` — full resource path printed by step 4 above
- `WIF_SERVICE_ACCOUNT` — `stadera-api-deployer@<project>.iam.gserviceaccount.com`
- `DATABASE_URL` — Neon Postgres connection string
- `GOOGLE_CLIENT_SECRET`

`stadera-api` reads only the variables above (see
`crates/api/src/config.rs`). Sessions are server-side opaque IDs in
the `sessions` table — no cookie-signing key needed. The Withings
`client_id` / `client_secret` / `token_key` are read only by
`stadera-jobs` and the `pair` binary, deployed separately in
M7-step2.

### Bootstrap order

The first deploy is chicken-and-egg: `GOOGLE_REDIRECT_URL` depends on
the Cloud Run service URL, which only exists after the first deploy.

1. Set everything except `GOOGLE_REDIRECT_URL` (or set a placeholder).
2. Trigger the workflow (push to `main` or manual `workflow_dispatch`).
3. Once deployed, copy the service URL:
   ```sh
   gcloud run services describe stadera-api --region=europe-west1 \
       --format='value(status.url)'
   ```
4. Set `GOOGLE_REDIRECT_URL=<URL>/auth/google/callback` as a GitHub
   variable, AND add it to the Google OAuth Console authorized
   redirect URIs.
5. Re-trigger the workflow so the new env var is picked up.

When you eventually move to a custom domain (`api.stadera.app`), the
URL becomes stable and step 5 isn't needed on subsequent infrastructure
churns.

## License

MIT
