# Terraform skeleton for the Stadera GCP infra.
#
# Scope of PR-A: declarative description of what already exists. After
# `terraform import` of the live resources, `terraform plan` should
# report "No changes". From this point on, IaC takes over: any
# modification to the declared resources happens via .tf edit + apply,
# not gcloud.
#
# What's NOT in this PR (deferred to PR-B):
#   - Secret Manager secrets (still in GitHub Actions for now)
#   - Dedicated runtime service accounts (Cloud Run service + job
#     run as the project's compute SA today)
#   - Image revision management (workflow keeps owning :tag rolls)
#
# Bootstrap order:
#   1. Create the state bucket once (out-of-band; chicken-and-egg with
#      Terraform's own backend). See terraform/README.md.
#   2. `terraform init` — downloads providers, configures backend.
#   3. `terraform import` for each existing resource (commands in
#      terraform/README.md).
#   4. `terraform plan` should show "No changes". If it shows updates,
#      tighten the .tf to match reality.

terraform {
  required_version = ">= 1.9.0"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 6.0"
    }
    google-beta = {
      source  = "hashicorp/google-beta"
      version = "~> 6.0"
    }
  }

  # State lives in a GCS bucket so the team (= Michele today, more
  # later) shares one source of truth and runs against the same plan.
  # `prefix` lets us version multiple environments under one bucket if
  # we ever split prod/staging.
  backend "gcs" {
    bucket = "stadera-tfstate"
    prefix = "prod"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

provider "google-beta" {
  project = var.project_id
  region  = var.region
}
