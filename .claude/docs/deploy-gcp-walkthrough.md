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

## References

- [stadera-web walkthrough](https://github.com/MicheleBellitti/stadera-web/blob/main/.claude/docs/deploy-gcp-walkthrough.md) — the long version of WIF
- [Cloud Run env vars + secrets](https://cloud.google.com/run/docs/configuring/services/environment-variables)
- [Distroless images](https://github.com/GoogleContainerTools/distroless) — what the runtime stage uses
- [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) — Rust dep caching for Docker
