# Stadera infra — Terraform

Declarative IaC for the GCP resources that back Stadera. PR-A scope:
mirror what already exists (created earlier with `gcloud`), import the
state, get to `plan = no changes`. Subsequent PRs evolve the infra
through `terraform apply`.

## What's managed here

- Artifact Registry repo (`stadera`)
- Service accounts (`stadera-api-deployer`, `stadera-web-deployer`,
  `stadera-sync-invoker`)
- IAM bindings on the project + per-resource
- Workload Identity Federation pool + 2 providers (one per GitHub repo)
- Cloud Run Service (`stadera-api`)
- Cloud Run Job (`stadera-sync`)
- Custom domain mappings (`api.stadera.org`, `app.stadera.org`)
- Cloud Scheduler trigger for the daily sync

## What's NOT managed here

- The image revisions running on Cloud Run — those are owned by the
  CI workflows (`.github/workflows/deploy.yml` in this repo and in
  `stadera-web`). Terraform pins the *shape* (resource limits, env
  var keys, ingress, scaling bounds); the workflow rolls *new
  revisions* with the latest image tag. The `lifecycle.ignore_changes
  = [template, ...]` blocks in `cloud_run.tf` enforce the split.
- Secret Manager secrets. PR-B introduces these and migrates the
  values currently passed inline by the workflow.
- Dedicated runtime service accounts. PR-B replaces the project's
  default compute SA with `stadera-api-runtime` (with
  `secretAccessor` on the secrets above).
- Neon Postgres. Provisioned out-of-band on Neon's console; we just
  consume `DATABASE_URL`.

## Bootstrap (one-time per environment)

Terraform's GCS backend can't bootstrap itself — the state bucket
must exist before `terraform init`. Create it manually:

```sh
PROJECT=stadera-494515
REGION=europe-west1

gcloud storage buckets create gs://stadera-tfstate \
    --project=$PROJECT --location=$REGION \
    --uniform-bucket-level-access

# Versioning so we can recover from accidentally bad applies.
gcloud storage buckets update gs://stadera-tfstate --versioning
```

Then:

```sh
cd terraform
terraform init   # downloads providers, configures backend
```

## Importing existing resources

Every resource declared in this directory already exists in GCP
(created via gcloud during M7-step1 / M7-step2). Run the import
commands below in order. After all imports, `terraform plan` should
show "No changes". If it shows updates, the .tf doesn't match
reality — adjust the .tf and re-plan.

```sh
PROJECT=stadera-494515
REGION=europe-west1
DOMAIN=stadera.org

# 1. Artifact Registry
terraform import google_artifact_registry_repository.stadera \
    projects/$PROJECT/locations/$REGION/repositories/stadera

# 2. Service accounts
terraform import google_service_account.backend_deployer \
    projects/$PROJECT/serviceAccounts/stadera-api-deployer@$PROJECT.iam.gserviceaccount.com
terraform import google_service_account.frontend_deployer \
    projects/$PROJECT/serviceAccounts/stadera-web-deployer@$PROJECT.iam.gserviceaccount.com
terraform import google_service_account.sync_invoker \
    projects/$PROJECT/serviceAccounts/stadera-sync-invoker@$PROJECT.iam.gserviceaccount.com

# 3. Project-level IAM bindings (member-style, one per binding)
# Format: <project> <role> <member>
terraform import 'google_project_iam_member.backend_deployer_run_admin' \
    "$PROJECT roles/run.admin serviceAccount:stadera-api-deployer@$PROJECT.iam.gserviceaccount.com"
terraform import 'google_project_iam_member.backend_deployer_ar_writer' \
    "$PROJECT roles/artifactregistry.writer serviceAccount:stadera-api-deployer@$PROJECT.iam.gserviceaccount.com"
terraform import 'google_project_iam_member.backend_deployer_sa_user' \
    "$PROJECT roles/iam.serviceAccountUser serviceAccount:stadera-api-deployer@$PROJECT.iam.gserviceaccount.com"
terraform import 'google_project_iam_member.frontend_deployer_run_admin' \
    "$PROJECT roles/run.admin serviceAccount:stadera-web-deployer@$PROJECT.iam.gserviceaccount.com"
terraform import 'google_project_iam_member.frontend_deployer_ar_writer' \
    "$PROJECT roles/artifactregistry.writer serviceAccount:stadera-web-deployer@$PROJECT.iam.gserviceaccount.com"
terraform import 'google_project_iam_member.frontend_deployer_sa_user' \
    "$PROJECT roles/iam.serviceAccountUser serviceAccount:stadera-web-deployer@$PROJECT.iam.gserviceaccount.com"

# 4. WIF pool + providers
terraform import google_iam_workload_identity_pool.github \
    projects/$PROJECT/locations/global/workloadIdentityPools/github-pool
terraform import google_iam_workload_identity_pool_provider.backend \
    projects/$PROJECT/locations/global/workloadIdentityPools/github-pool/providers/stadera-api-provider
terraform import google_iam_workload_identity_pool_provider.frontend \
    projects/$PROJECT/locations/global/workloadIdentityPools/github-pool/providers/github-provider

# 5. WIF principalSet bindings (on the SA)
PROJECT_NUMBER=$(gcloud projects describe $PROJECT --format='value(projectNumber)')
POOL_PATH="projects/$PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool"

terraform import google_service_account_iam_member.backend_wif_binding \
    "projects/$PROJECT/serviceAccounts/stadera-api-deployer@$PROJECT.iam.gserviceaccount.com roles/iam.workloadIdentityUser principalSet://iam.googleapis.com/$POOL_PATH/attribute.repository/MicheleBellitti/Stadera"
terraform import google_service_account_iam_member.frontend_wif_binding \
    "projects/$PROJECT/serviceAccounts/stadera-web-deployer@$PROJECT.iam.gserviceaccount.com roles/iam.workloadIdentityUser principalSet://iam.googleapis.com/$POOL_PATH/attribute.repository/MicheleBellitti/stadera-web"

# 6. Cloud Run Service + public-invoker binding
terraform import google_cloud_run_v2_service.api \
    projects/$PROJECT/locations/$REGION/services/stadera-api
terraform import google_cloud_run_v2_service_iam_member.api_public \
    "$PROJECT/$REGION/stadera-api roles/run.invoker allUsers"

# 7. Cloud Run Job + invoker SA binding
terraform import google_cloud_run_v2_job.sync \
    projects/$PROJECT/locations/$REGION/jobs/stadera-sync
terraform import google_cloud_run_v2_job_iam_member.sync_invoker \
    "$PROJECT/$REGION/stadera-sync roles/run.invoker serviceAccount:stadera-sync-invoker@$PROJECT.iam.gserviceaccount.com"

# 8. Domain mappings
terraform import google_cloud_run_domain_mapping.api \
    locations/$REGION/namespaces/$PROJECT/domainmappings/api.$DOMAIN
terraform import google_cloud_run_domain_mapping.app \
    locations/$REGION/namespaces/$PROJECT/domainmappings/app.$DOMAIN

# 9. Cloud Scheduler
terraform import google_cloud_scheduler_job.sync_daily \
    projects/$PROJECT/locations/$REGION/jobs/stadera-sync-daily
```

After every section, run `terraform plan` and verify the planned
changes are empty (or only minor formatting differences in computed
fields). If real changes show up, the .tf in this directory needs
adjustment to match reality — do not `terraform apply` blindly.

## Daily workflow

Once import is complete:

```sh
# Make a change to a .tf file
terraform plan      # preview
terraform apply     # commit it to GCP

# Inspect current state
terraform state list
terraform state show google_cloud_run_v2_service.api
```

For changes the workflow ALSO touches (e.g. you want to bump
`min_instance_count`), edit the .tf, plan, apply. Subsequent CI
deploys won't undo it because of the `ignore_changes` block — they
only roll the image, not the shape.

## What's next (PR-B)

- Move `DATABASE_URL`, `GOOGLE_CLIENT_SECRET`, `WITHINGS_*`,
  `WITHINGS_TOKEN_KEY` into Secret Manager.
- Replace the project's default compute SA on Cloud Run Service / Job
  with a dedicated `stadera-api-runtime` SA.
- Grant `secretAccessor` on the secrets to the runtime SA.
- Update the GitHub workflow to use `--update-secrets=KEY=secret-name:latest`
  instead of `env_vars: KEY=$secretvalue` (no more secret values
  flowing through the runner).
