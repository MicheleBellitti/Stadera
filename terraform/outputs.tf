# Outputs used by humans (mostly during bootstrap) and by other tools
# that may read the state file.

output "api_url" {
  description = "Public URL of the stadera-api Cloud Run service (auto-generated *.run.app)."
  value       = google_cloud_run_v2_service.api.uri
}

output "api_custom_domain" {
  description = "Custom domain for the API."
  value       = "https://api.${var.domain}"
}

output "app_custom_domain" {
  description = "Custom domain for the frontend."
  value       = "https://app.${var.domain}"
}

output "wif_provider_backend" {
  description = "Full WIF provider resource path for the backend repo. Paste into GitHub secrets as WIF_PROVIDER."
  value       = google_iam_workload_identity_pool_provider.backend.name
}

output "wif_provider_frontend" {
  description = "Full WIF provider resource path for the frontend repo."
  value       = google_iam_workload_identity_pool_provider.frontend.name
}

output "deployer_sa_backend" {
  description = "Backend deployer SA email. Paste into GitHub secrets as WIF_SERVICE_ACCOUNT."
  value       = google_service_account.backend_deployer.email
}

output "deployer_sa_frontend" {
  description = "Frontend deployer SA email."
  value       = google_service_account.frontend_deployer.email
}

output "sync_invoker_sa" {
  description = "SA used by Cloud Scheduler to invoke the daily sync."
  value       = google_service_account.sync_invoker.email
}

output "artifact_registry_repo" {
  description = "Full Artifact Registry path for the shared image repo."
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.stadera.repository_id}"
}
