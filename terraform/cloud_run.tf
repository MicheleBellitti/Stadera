# Cloud Run Service (stadera-api), Cloud Run Job (stadera-sync), and the
# domain mappings for both.
#
# The SHAPE of these resources is owned by Terraform: ports, scaling
# bounds, resource limits, env-var keys. The IMAGE TAG is owned by the
# CI workflow: each push to main rolls a new revision via
# `gcloud run services update --image=…:sha` (or the deploy-cloudrun
# action). Terraform would otherwise fight CI for ownership of the
# image attribute.
#
# The `lifecycle.ignore_changes = [template]` block below tells
# Terraform to leave the template alone after first creation — exactly
# the "shape vs revision" split.

# ---- Cloud Run Service (stadera-api) ----------------------------------

resource "google_cloud_run_v2_service" "api" {
  project  = var.project_id
  location = var.region
  name     = local.service_name

  ingress = "INGRESS_TRAFFIC_ALL"

  template {
    scaling {
      min_instance_count = 0
      max_instance_count = 2
    }

    timeout = "60s"

    containers {
      # Image is set by CI on each deploy. Terraform pins the SHAPE,
      # not the revision (see lifecycle below). The placeholder
      # `:bootstrap` exists only so Terraform has something to apply
      # on first import — CI overwrites it on the next push.
      image = "${var.region}-docker.pkg.dev/${var.project_id}/${local.ar_repo}/${local.service_name}:bootstrap"

      ports {
        container_port = 8080
      }

      resources {
        limits = {
          cpu    = "1"
          memory = "512Mi"
        }
      }

      # Env vars are managed by CI (`deploy-cloudrun` action's
      # env_vars: block) — see ignore_changes below. Listed here only
      # for documentation of what the workload reads.
      #
      # FRONTEND_ORIGIN, COOKIE_DOMAIN, GOOGLE_CLIENT_ID,
      # GOOGLE_REDIRECT_URL, COOKIE_SECURE, DATABASE_URL,
      # GOOGLE_CLIENT_SECRET
    }
  }

  traffic {
    type    = "TRAFFIC_TARGET_ALLOCATION_TYPE_LATEST"
    percent = 100
  }

  lifecycle {
    ignore_changes = [
      # CI owns the revision content. Terraform owns the service
      # existence + ingress config + traffic split.
      template,
      client,
      client_version,
    ]
  }
}

resource "google_cloud_run_v2_service_iam_member" "api_public" {
  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_service.api.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# ---- Cloud Run Job (stadera-sync) -------------------------------------

resource "google_cloud_run_v2_job" "sync" {
  project  = var.project_id
  location = var.region
  name     = local.job_name

  template {
    template {
      timeout     = "300s"
      max_retries = 2

      containers {
        image   = "${var.region}-docker.pkg.dev/${var.project_id}/${local.ar_repo}/${local.service_name}:bootstrap"
        command = ["/usr/local/bin/stadera-jobs"]
        args    = ["sync", "--user-email", var.sync_user_email]

        resources {
          limits = {
            cpu    = "1"
            memory = "512Mi"
          }
        }
      }
    }
  }

  lifecycle {
    ignore_changes = [
      template,
      client,
      client_version,
    ]
  }
}

# ---- Sync invoker permission on the Job (not on the project) ----------

resource "google_cloud_run_v2_job_iam_member" "sync_invoker" {
  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_job.sync.name
  role     = "roles/run.invoker"
  member   = "serviceAccount:${google_service_account.sync_invoker.email}"
}

# ---- Custom domain mappings -------------------------------------------

resource "google_cloud_run_domain_mapping" "api" {
  provider = google-beta

  project  = var.project_id
  location = var.region
  name     = "api.${var.domain}"

  metadata {
    namespace = var.project_id
  }

  spec {
    route_name = google_cloud_run_v2_service.api.name
  }
}

# Frontend service is deployed by the stadera-web repo workflow, but
# the domain mapping lives in the same project — IaC ownership belongs
# here. We reference it by name (the resource itself isn't managed by
# this Terraform module).
resource "google_cloud_run_domain_mapping" "app" {
  provider = google-beta

  project  = var.project_id
  location = var.region
  name     = "app.${var.domain}"

  metadata {
    namespace = var.project_id
  }

  spec {
    route_name = "stadera-web"
  }
}
