# Backend Cloud Run deploy — GCP walkthrough

Step-by-step explanation of the `gcloud` commands in
[the README's Deploy section](../../README.md#deploy).

If you've never set up Workload Identity Federation before,
**read [stadera-web's walkthrough](https://github.com/MicheleBellitti/stadera-web/blob/main/.claude/docs/deploy-gcp-walkthrough.md)
first** — it covers the OIDC ↔ STS ↔ IAM ↔ SA flow in depth, the
mental model of pools and providers, and the diagnostics for the
common failure modes. This doc only covers what's *different* for the
backend.

## What's reused from the frontend setup

After running the frontend walkthrough, the following GCP resources
already exist:

| Resource | Name | Reuse for backend? |
|---|---|---|
| Artifact Registry repo | `stadera` (in `europe-west1`) | ✅ same repo, different image name (`stadera-api` vs `stadera-web`) |
| Workload Identity Pool | `github-pool` | ✅ same pool, new provider inside it |

What's *new* for the backend:

| Resource | Name | Why |
|---|---|---|
| Service account | `stadera-api-deployer` | separate identity per service, so blast radius of a compromised CI is limited to one repo |
| WIF Provider | `stadera-api-provider` | scoped to `MicheleBellitti/Stadera` (the existing `github-provider` is locked to `MicheleBellitti/stadera-web`) |
| Cloud Run Service | `stadera-api` | the actual runtime |

## Why a new provider per repo

You *could* update the existing provider's `attribute-condition` to
allow both repos, e.g.:

```cel
assertion.repository in ['MicheleBellitti/stadera-web', 'MicheleBellitti/Stadera']
```

But that's a single point of failure: if you tighten or break it for
one repo, you break the other. Two providers cost nothing (no quota,
no money) and let each repo's deploy auth be debugged independently.

## Step-by-step

The README's setup script runs these in order. Each is a one-liner;
the comments below say what it does and why.

### Service account

```sh
gcloud iam service-accounts create stadera-api-deployer --project=$PROJECT
```

Creates a non-human GCP identity. Same idea as the frontend's
`stadera-web-deployer` — separate from your user account, separate
permissions, easy to rotate or revoke.

### Project-level IAM bindings

```sh
gcloud projects add-iam-policy-binding $PROJECT \
    --member="serviceAccount:$SA_EMAIL" --role=roles/run.admin
gcloud projects add-iam-policy-binding $PROJECT \
    --member="serviceAccount:$SA_EMAIL" --role=roles/artifactregistry.writer
gcloud projects add-iam-policy-binding $PROJECT \
    --member="serviceAccount:$SA_EMAIL" --role=roles/iam.serviceAccountUser
```

The same three roles the frontend deployer needs:

- **`run.admin`** — create / update / delete the Cloud Run service.
- **`artifactregistry.writer`** — push images.
- **`iam.serviceAccountUser`** — necessary to deploy a service that
  runs as a *different* SA (the runtime SA). Without it, deploy fails
  with "Permission iam.serviceaccounts.actAs denied".

### New WIF provider

```sh
gcloud iam workload-identity-pools providers create-oidc $PROVIDER \
    --location=global --workload-identity-pool=$POOL --project=$PROJECT \
    --display-name="GitHub Actions (stadera backend)" \
    --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
    --attribute-condition="assertion.repository=='$GITHUB_REPO'" \
    --issuer-uri="https://token.actions.githubusercontent.com"
```

Same shape as the frontend's `github-provider`, only:

- `--display-name` distinguishes them in the GCP Console.
- `--attribute-condition` pins this provider to **the backend repo
  only**.

The two providers share the pool but are independent objects — you can
delete `stadera-api-provider` without touching the frontend's auth.

### Bind the repo to the SA

```sh
POOL_ID=$(gcloud iam workload-identity-pools describe $POOL \
    --location=global --project=$PROJECT --format='value(name)')

gcloud iam service-accounts add-iam-policy-binding $SA_EMAIL \
    --project=$PROJECT --role=roles/iam.workloadIdentityUser \
    --member="principalSet://iam.googleapis.com/$POOL_ID/attribute.repository/$GITHUB_REPO"
```

The binding lives **on the SA**, not on the pool or provider — because
*who can impersonate this SA* is a property of the SA itself.

The `principalSet://...attribute.repository/MicheleBellitti/Stadera`
URI is matched against the OIDC token attributes that pass the
provider's `attribute-condition`. So the chain is:

1. GitHub OIDC token arrives at GCP STS.
2. STS routes it to **the provider** that issued the credential
   configuration (configured via the `WIF_PROVIDER` GitHub secret).
3. Provider's `attribute-condition` filters out tokens from the wrong
   repo. Only `MicheleBellitti/Stadera` tokens pass.
4. STS issues a federated token tagged with the mapped attributes.
5. The federated token tries to impersonate `stadera-api-deployer`.
6. The SA's IAM policy is checked: does it have a binding for the
   principalSet that matches this attribute set? Yes → granted.
7. Deploy proceeds.

## Cloud Run runtime SA: a thing we're deferring

Right now the deployed Cloud Run service runs as the project's default
compute SA (`<projectnum>-compute@developer.gserviceaccount.com`).
That works because:

- The service doesn't need GCP API access at runtime — only DB
  (Neon, internet) and Withings API (internet).
- Env vars are set inline by the deploy workflow (no Secret Manager).

When we migrate to Secret Manager (M7-step3 / Terraform), the runtime
SA needs `secretmanager.secretAccessor` on the specific secrets.
That's the moment to introduce a dedicated `stadera-api-runtime` SA
with minimal grants. For step1, default compute SA is fine.

## After the first deploy

The deploy workflow needs `GOOGLE_REDIRECT_URL` as a GitHub variable,
and that URL doesn't exist until Cloud Run gives you one. Bootstrap
order:

1. Run the workflow with everything else configured. The
   redirect-URL-dependent OAuth flow won't work, but `/health` will.
2. `gcloud run services describe stadera-api --region=$REGION
    --format='value(status.url)'` → e.g.
   `https://stadera-api-XXX.europe-west1.run.app`.
3. Set `GOOGLE_REDIRECT_URL=https://stadera-api-XXX.europe-west1.run.app/auth/google/callback`
   as a GitHub variable.
4. Add the same URL to the Google OAuth Console's authorized redirect
   URIs (under your OAuth client).
5. Push a trivial commit (or trigger `workflow_dispatch`) to redeploy
   with the new env var.

You also need to set `FRONTEND_ORIGIN` to the deployed frontend's
Cloud Run URL. The frontend's `BACKEND_API_URL` becomes this
backend's URL. The two get coupled at first deploy, then both are
stable until you delete and recreate.

A custom domain (`api.stadera.app` + `app.stadera.app`) breaks the
coupling: URLs become stable across infrastructure churn.

## Smoke test

After deploy, hit the health endpoint:

```sh
URL=$(gcloud run services describe stadera-api --region=$REGION \
    --format='value(status.url)')
curl -i $URL/health
# Expected: HTTP/2 200, body { "status": "ok" }
```

For the OpenAPI spec:

```sh
curl $URL/api-docs/openapi.json | jq .info
# Expected: { "title": "Stadera API", "version": "0.1.0", ... }
```

If you get a 5xx, check Cloud Logging:

```sh
gcloud logging read \
    "resource.type=cloud_run_revision AND resource.labels.service_name=stadera-api" \
    --limit=50 --format='value(textPayload)' --project=$PROJECT
```

## M7-step2: Cloud Run Job + Cloud Scheduler

The same Docker image that serves `stadera-api` also contains
`stadera-jobs`. We deploy it as a **Cloud Run Job** (one-shot
execution, not a long-running service) and schedule daily invocations
via **Cloud Scheduler**.

### Why a Job and not just a Service

| | Service | Job |
|---|---|---|
| Lifecycle | always-on (or scale-to-zero with cold start on request) | exits when the binary returns |
| Triggered by | HTTP requests | manual invoke / Scheduler / Eventarc |
| Billing | per request-second (idle = $0 with min-instances=0) | per execution duration only |
| Suited for | API serving | batch / ETL / cron |

The sync is a 5-30 s batch task that runs once a day. Modeling it as
a service would force us into either always-on (waste) or
HTTP-triggering-itself (silly). Jobs map perfectly.

### Workflow side: Cloud Run Job deploy

The `Deploy Cloud Run Job` step in `.github/workflows/deploy.yml`:

1. Writes a temp YAML file with `DATABASE_URL`, `WITHINGS_*` env vars.
2. Calls `gcloud run jobs deploy` — creates the Job on first run,
   updates the image + env on subsequent runs.
3. Sets `--command=/usr/local/bin/stadera-jobs` and
   `--args=sync,--user-email,${SYNC_USER_EMAIL}`. The comma-separated
   `--args` becomes `argv` inside the container —
   `["sync", "--user-email", "user@..."]`.
4. Deletes the temp YAML.

Why `--env-vars-file` and not inline `--set-env-vars`: gcloud's flag
parser splits the inline form on commas, so any value containing a
comma (Postgres URLs sometimes do) breaks. The file form is exact.

### Cloud Scheduler: anatomy of a daily trigger

```sh
gcloud scheduler jobs create http stadera-sync-daily \
    --location=$REGION --project=$PROJECT \
    --schedule="0 6 * * *" \
    --time-zone="Europe/Rome" \
    --uri="https://$REGION-run.googleapis.com/apis/run.googleapis.com/v1/namespaces/$PROJECT/jobs/$JOB:run" \
    --http-method=POST \
    --oidc-service-account-email=$INVOKER_SA_EMAIL \
    --oidc-token-audience="https://$REGION-run.googleapis.com/"
```

Reading flag by flag:

- **`--schedule="0 6 * * *"`** — standard cron. Minute 0, hour 6,
  any day of month, any month, any day of week. Daily 06:00.
- **`--time-zone="Europe/Rome"`** — interpreted in this TZ, so
  daylight savings shifts are absorbed automatically. Without this
  the schedule is UTC and you'd be triggering at 07:00 / 08:00
  depending on the season.
- **`--uri=…/jobs/$JOB:run`** — the Cloud Run Admin API endpoint
  that triggers a job execution. The `:run` suffix is the magic;
  hitting the bare `…/jobs/$JOB` returns metadata, not an
  execution.
- **`--http-method=POST`** — required by the `:run` endpoint.
- **`--oidc-service-account-email`** — Scheduler signs the outgoing
  request with an OIDC token *minted as this service account*. Cloud
  Run sees the token, looks up the SA's IAM, decides yes/no.
- **`--oidc-token-audience`** — the audience claim Scheduler bakes
  into the token. Cloud Run requires it match its own URL.

### IAM minimum for the invoker SA

```sh
gcloud run jobs add-iam-policy-binding $JOB \
    --region=$REGION --project=$PROJECT \
    --member="serviceAccount:$INVOKER_SA_EMAIL" \
    --role=roles/run.invoker
```

Note this binds **on the Job**, not on the project. The SA can invoke
*this* job, nothing else. If a future malicious workflow steals the
SA, the worst it can do is re-trigger the sync.

### Smoke test the chain

Manually run the job:

```sh
gcloud run jobs execute $JOB --region=$REGION --wait --project=$PROJECT
# → Execution [stadera-sync-...]: SUCCEEDED (took 12s)
```

Trigger via Scheduler (without waiting for the cron):

```sh
gcloud scheduler jobs run stadera-sync-daily --location=$REGION --project=$PROJECT
```

Then check the execution:

```sh
gcloud run jobs executions list --job=$JOB --region=$REGION --project=$PROJECT --limit=5
```

The most recent execution should be `SUCCEEDED`. If it's `FAILED`,
inspect logs via `gcloud beta run jobs executions describe <id>` or
the Cloud Console "Executions" tab on the Job.

### Common failure: missing `WITHINGS_TOKEN_KEY` mismatch

The token key encrypts Withings refresh tokens at rest. If the value
in GitHub Secrets differs from what `make pair` used locally, sync
fails to decrypt and bails with `decryption failed`. Fix: rerun
`make pair` with the production key (or rotate every paired user).

## Custom domain mapping (cookie cross-subdomain)

The auto-generated `*.run.app` URLs work, but session cookies don't
cross between two `*.run.app` subdomains (browsers won't share cookies
across hosts that aren't a parent/child relationship through the
public-suffix list — and `run.app` is a public suffix). With FE on
`stadera-web-X.run.app` and BE on `stadera-api-Y.run.app`, the BE-set
cookie is invisible to the FE host → /me from FE always sees 401.

A custom domain solves it:

```sh
DOMAIN=stadera.org   # owned by you, DNS managed by Cloudflare or similar

gcloud beta run domain-mappings create \
    --service=stadera-api --domain="api.$DOMAIN" \
    --region=$REGION --project=$PROJECT

gcloud beta run domain-mappings create \
    --service=stadera-web --domain="app.$DOMAIN" \
    --region=$REGION --project=$PROJECT
```

`gcloud` prints CNAME records to add to your DNS provider:

```
api.stadera.org   CNAME   ghs.googlehosted.com
app.stadera.org   CNAME   ghs.googlehosted.com
```

DNS propagation: ~5-30 min. Cloud Run auto-provisions Let's Encrypt
TLS certs once the CNAMEs resolve correctly (~5 min more).

After DNS + TLS:

1. Set GitHub Variable `COOKIE_DOMAIN=.stadera.org`. The leading dot
   isn't strictly required by RFC 6265bis but many HTTP clients still
   prefer it for "shared across all subdomains" semantics.
2. Set `FRONTEND_ORIGIN=https://app.stadera.org` and
   `GOOGLE_REDIRECT_URL=https://api.stadera.org/auth/google/callback`.
3. Add `https://api.stadera.org/auth/google/callback` to the Google
   OAuth Console authorized redirect URIs.
4. Update the frontend's `BACKEND_API_URL` GitHub Variable to
   `https://api.stadera.org`.
5. Re-trigger both repos' deploy workflows so the new env values are
   picked up.

After that, sign-in on `https://app.stadera.org` sets a cookie with
`Domain=.stadera.org` → browser sends it to both `app.*` and `api.*`
→ the FE's server-side `/me` gate works and the user lands on
`/dashboard`.

## References

- [stadera-web walkthrough](https://github.com/MicheleBellitti/stadera-web/blob/main/.claude/docs/deploy-gcp-walkthrough.md) — the long version of WIF
- [Cloud Run env vars + secrets](https://cloud.google.com/run/docs/configuring/services/environment-variables)
- [Cloud Run Jobs reference](https://cloud.google.com/run/docs/create-jobs)
- [Cloud Scheduler with OIDC](https://cloud.google.com/scheduler/docs/http-target-auth)
- [Distroless images](https://github.com/GoogleContainerTools/distroless) — what the runtime stage uses
- [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) — Rust dep caching for Docker
