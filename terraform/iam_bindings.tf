# IAM bindings for the deployer + invoker service accounts.
#
# We use `google_project_iam_member` (additive single-binding) instead
# of `_iam_binding` (replaces all members for that role) because the
# project has other grants (GCP-managed agents, our own user) we don't
# want Terraform to clobber. `_member` is the safe default for
# greenfield IaC over a hand-managed project.

# ---- Backend deployer -------------------------------------------------

resource "google_project_iam_member" "backend_deployer_run_admin" {
  project = var.project_id
  role    = "roles/run.admin"
  member  = "serviceAccount:${google_service_account.backend_deployer.email}"
}

resource "google_project_iam_member" "backend_deployer_ar_writer" {
  project = var.project_id
  role    = "roles/artifactregistry.writer"
  member  = "serviceAccount:${google_service_account.backend_deployer.email}"
}

# Required so the deployer can pass-through the runtime SA when
# deploying the Cloud Run service / job (Cloud Run requires `actAs`
# on whatever SA the workload will run as).
resource "google_project_iam_member" "backend_deployer_sa_user" {
  project = var.project_id
  role    = "roles/iam.serviceAccountUser"
  member  = "serviceAccount:${google_service_account.backend_deployer.email}"
}

# ---- Frontend deployer ------------------------------------------------

resource "google_project_iam_member" "frontend_deployer_run_admin" {
  project = var.project_id
  role    = "roles/run.admin"
  member  = "serviceAccount:${google_service_account.frontend_deployer.email}"
}

resource "google_project_iam_member" "frontend_deployer_ar_writer" {
  project = var.project_id
  role    = "roles/artifactregistry.writer"
  member  = "serviceAccount:${google_service_account.frontend_deployer.email}"
}

resource "google_project_iam_member" "frontend_deployer_sa_user" {
  project = var.project_id
  role    = "roles/iam.serviceAccountUser"
  member  = "serviceAccount:${google_service_account.frontend_deployer.email}"
}

# ---- Sync invoker -----------------------------------------------------

# Bound on the Job (not on the project) so the SA can ONLY invoke this
# specific job. See cloud_run_job.tf for the binding resource.
