# Derived values. Keep these as locals (not vars) so they can't be
# overridden — they're invariants of the deployment, not knobs.

locals {
  # Service / job names — match the strings used by the CI workflows.
  service_name = "stadera-api"
  job_name     = "stadera-sync"

  # SAs used during deploys. Naming convention: <service>-deployer for
  # IaC / CI principals, <service>-runtime for the SA the workload runs
  # as (PR-B introduces dedicated runtimes; today both run as the
  # project's compute SA).
  sa_backend_deployer  = "stadera-api-deployer"
  sa_frontend_deployer = "stadera-web-deployer"
  sa_sync_invoker      = "stadera-sync-invoker"

  # Workload Identity Federation pool — shared across both repos.
  wif_pool_id = "github-pool"

  # Two providers, one per repo, so a bad attribute_condition on one
  # never disrupts deploys of the other.
  wif_provider_backend  = "stadera-api-provider"
  wif_provider_frontend = "github-provider"

  # Artifact Registry repo, shared between backend and frontend image
  # streams.
  ar_repo = "stadera"

  # Cloud Scheduler invokes the Cloud Run Job daily at 06:00 Europe/Rome
  # — early enough that the morning weighing is in Withings cloud, late
  # enough that the user can have notifications fire by ~6:05.
  scheduler_cron = "0 6 * * *"
  scheduler_tz   = "Europe/Rome"

  # GCP project number — needed by WIF resource paths. Looked up from
  # the data source below to avoid hardcoding.
}

# Project number is required in the WIF principalSet URI.
data "google_project" "this" {
  project_id = var.project_id
}
