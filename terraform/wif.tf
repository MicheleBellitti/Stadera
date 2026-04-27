# Workload Identity Federation: GitHub Actions → impersonate deployer SAs.
#
# One pool, two providers (one per repo). Each provider's
# `attribute_condition` pins the OIDC tokens to a specific repo so
# tokens minted for repo X can never impersonate the SA for repo Y.

resource "google_iam_workload_identity_pool" "github" {
  project                   = var.project_id
  workload_identity_pool_id = local.wif_pool_id
  display_name              = "GitHub"
  description               = "Federated identities from GitHub Actions tokens."
}

# ---- Backend repo provider --------------------------------------------

resource "google_iam_workload_identity_pool_provider" "backend" {
  project                            = var.project_id
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = local.wif_provider_backend
  display_name                       = "GitHub Actions (stadera backend)"

  attribute_mapping = {
    "google.subject"       = "assertion.sub"
    "attribute.repository" = "assertion.repository"
  }

  attribute_condition = "assertion.repository=='${var.github_repo_backend}'"

  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
}

# ---- Frontend repo provider -------------------------------------------

resource "google_iam_workload_identity_pool_provider" "frontend" {
  project                            = var.project_id
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = local.wif_provider_frontend
  display_name                       = "GitHub Actions (stadera-web)"

  attribute_mapping = {
    "google.subject"       = "assertion.sub"
    "attribute.repository" = "assertion.repository"
  }

  attribute_condition = "assertion.repository=='${var.github_repo_frontend}'"

  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
}

# ---- Bind GitHub identities → deployer SAs ----------------------------

# Each binding scopes "principalSet of pool members with this attribute"
# to "can impersonate this specific SA via roles/iam.workloadIdentityUser".
# The principalSet URI form is mandated by GCP; the project number
# component comes from the data source.

locals {
  pool_resource_name = "projects/${data.google_project.this.number}/locations/global/workloadIdentityPools/${google_iam_workload_identity_pool.github.workload_identity_pool_id}"
}

resource "google_service_account_iam_member" "backend_wif_binding" {
  service_account_id = google_service_account.backend_deployer.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${local.pool_resource_name}/attribute.repository/${var.github_repo_backend}"
}

resource "google_service_account_iam_member" "frontend_wif_binding" {
  service_account_id = google_service_account.frontend_deployer.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${local.pool_resource_name}/attribute.repository/${var.github_repo_frontend}"
}
