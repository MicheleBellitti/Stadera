# Service accounts used by deploys (CI) and by Cloud Scheduler.
#
# Naming convention recap:
# - `*-deployer` : assumed by GitHub Actions through WIF, has the
#   project-level grants needed to push images and roll Cloud Run
#   revisions.
# - `*-invoker`  : assumed by Cloud Scheduler when triggering the
#   daily sync, has only `run.invoker` on the specific Job.
#
# Cloud Run Service / Job currently run as the project's compute SA
# (no `runtime` SA declared here yet). PR-B introduces a dedicated
# stadera-api-runtime once we move secrets to Secret Manager — that's
# when the runtime SA needs explicit `secretAccessor` grants.

resource "google_service_account" "backend_deployer" {
  project      = var.project_id
  account_id   = local.sa_backend_deployer
  display_name = "Stadera API deployer"
  description  = "Used by GitHub Actions on MicheleBellitti/Stadera to deploy stadera-api + stadera-sync."
}

resource "google_service_account" "frontend_deployer" {
  project      = var.project_id
  account_id   = local.sa_frontend_deployer
  display_name = "Stadera web deployer"
  description  = "Used by GitHub Actions on MicheleBellitti/stadera-web to deploy stadera-web."
}

resource "google_service_account" "sync_invoker" {
  project      = var.project_id
  account_id   = local.sa_sync_invoker
  display_name = "Stadera sync invoker"
  description  = "Used by Cloud Scheduler to invoke the daily sync Cloud Run Job."
}
