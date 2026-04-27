# Stadera infra — Terraform

Declarative IaC for the GCP resources that back Stadera. Scope:
mirror what already exists in your project (created earlier via
`gcloud`), import the state, then evolve infra through
`terraform apply`.

## What's managed here

- Artifact Registry repo
- Service accounts (deployer × 2 + sync invoker)
- IAM bindings (project + per-resource)
- Workload Identity Federation pool + provider per repo
- Cloud Run Service (`stadera-api`)
- Cloud Run Job (`stadera-sync`)
- Custom domain mappings (`api.<domain>`, `app.<domain>`)
- Cloud Scheduler trigger for the daily sync

## What's NOT managed here

- **Image revisions** running on Cloud Run — owned by the CI
  workflows. Terraform pins the *shape* (resource limits, env var
  keys, ingress, scaling); the workflow rolls *new revisions* with
  the latest image tag. The `lifecycle.ignore_changes = [template, ...]`
  blocks in `cloud_run.tf` enforce the split.
- **Secret Manager secrets** — deferred to PR-B.
- **Dedicated runtime service accounts** — deferred to PR-B.
- **Neon Postgres** — provisioned out-of-band on Neon's console; we
  just consume `DATABASE_URL`.

## Quickstart

1. **Project-specific values** — copy
   `terraform.tfvars.example` to `terraform.tfvars` (gitignored)
   and fill in `project_id`, `github_repo_*`, `domain`,
   `sync_user_email`.
2. **State bucket** (one-time bootstrap, can't be self-managed by
   Terraform):

   ```sh
   PROJECT=<your project id>
   REGION=europe-west1
   gcloud storage buckets create gs://${PROJECT}-tfstate \
       --project=$PROJECT --location=$REGION \
       --uniform-bucket-level-access
   gcloud storage buckets update gs://${PROJECT}-tfstate --versioning
   ```

   Update `main.tf` `backend "gcs"` block with your bucket name if
   different from `stadera-tfstate`.
3. **Init**:

   ```sh
   terraform init
   ```
4. **Import existing resources** if your GCP project already has
   them (created via `gcloud` earlier). The import sequence — ~25
   commands with full resource paths — is operational and lives
   outside this public repo.
5. **Plan + apply**:

   ```sh
   terraform plan      # preview, expect "No changes" after import
   terraform apply
   ```

## Daily workflow

```sh
# Make a change in .tf
terraform plan      # preview
terraform apply     # commit it to GCP

# Inspect current state
terraform state list
terraform state show google_cloud_run_v2_service.api
```

For changes the deploy workflow ALSO touches (e.g. you want to bump
`min_instance_count`), edit the `.tf`, plan, apply. Subsequent CI
deploys won't undo it because of the `ignore_changes` block — they
only roll the image, not the shape.

## What's next (PR-B)

- Move `DATABASE_URL`, `GOOGLE_CLIENT_SECRET`, `WITHINGS_*` into
  Secret Manager.
- Replace the project's default compute SA on Cloud Run Service /
  Job with a dedicated `stadera-api-runtime` SA.
- Grant `secretAccessor` on the secrets to the runtime SA.
- Update the GitHub workflow to use `--update-secrets=KEY=secret-name:latest`
  instead of `env_vars: KEY=$secretvalue` (no more secret values
  flowing through the runner).
