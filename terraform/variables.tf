# Input variables. Default values match the live infra so
# `terraform apply` without overrides reproduces what's running.

variable "project_id" {
  description = "GCP project ID."
  type        = string
  default     = "stadera-494515"
}

variable "region" {
  description = "Default region for regional resources (Cloud Run, Artifact Registry, …)."
  type        = string
  default     = "europe-west1"
}

variable "github_repo_backend" {
  description = "GitHub repo allowed to deploy the backend via WIF (owner/repo)."
  type        = string
  default     = "MicheleBellitti/Stadera"
}

variable "github_repo_frontend" {
  description = "GitHub repo allowed to deploy the frontend via WIF."
  type        = string
  default     = "MicheleBellitti/stadera-web"
}

variable "domain" {
  description = "Custom domain root. Subdomains api/app are mapped under it."
  type        = string
  default     = "stadera.org"
}

variable "sync_user_email" {
  description = "Email of the user whose Withings data the daily sync pulls."
  type        = string
  default     = "michelebellitti272@gmail.com"
  sensitive   = false
}
