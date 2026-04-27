# Input variables. No defaults for project-specific values — the
# public repo doesn't ship anyone's project ID or owner email. Pass
# them via `terraform.tfvars` (gitignored) or `-var key=value` on
# the command line. Generic infrastructure choices (region, domain
# pattern) keep their defaults.

variable "project_id" {
  description = "GCP project ID. Required."
  type        = string
}

variable "region" {
  description = "Default region for regional resources (Cloud Run, Artifact Registry, …)."
  type        = string
  default     = "europe-west1"
}

variable "github_repo_backend" {
  description = "GitHub repo allowed to deploy the backend via WIF (owner/repo). Required."
  type        = string
}

variable "github_repo_frontend" {
  description = "GitHub repo allowed to deploy the frontend via WIF (owner/repo). Required."
  type        = string
}

variable "domain" {
  description = "Custom domain root. Subdomains api/app are mapped under it. Required."
  type        = string
}

variable "sync_user_email" {
  description = "Email of the user whose Withings data the daily sync pulls. Required."
  type        = string
  sensitive   = true
}
