# Daily HTTP trigger that invokes the Cloud Run Job via the Cloud Run
# Admin API. The OIDC token Cloud Scheduler attaches is verified by
# Cloud Run; the audience must match the regional run.googleapis.com
# endpoint.

resource "google_cloud_scheduler_job" "sync_daily" {
  project   = var.project_id
  region    = var.region
  name      = "stadera-sync-daily"
  schedule  = local.scheduler_cron
  time_zone = local.scheduler_tz

  description = "Triggers stadera-sync Cloud Run Job daily at 06:00 Europe/Rome."

  http_target {
    http_method = "POST"
    uri         = "https://${var.region}-run.googleapis.com/apis/run.googleapis.com/v1/namespaces/${var.project_id}/jobs/${google_cloud_run_v2_job.sync.name}:run"

    oidc_token {
      service_account_email = google_service_account.sync_invoker.email
      audience              = "https://${var.region}-run.googleapis.com/"
    }
  }

  # No retry config: if a daily run fails, we'd rather see it in
  # logging than have Scheduler hammer Withings's rate limits with
  # retries. The Job itself has `max_retries = 2` in cloud_run.tf
  # which is enough.
}
