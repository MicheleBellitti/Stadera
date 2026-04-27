# Single Docker repo shared between backend and frontend image streams.
# Both CI workflows push to <region>-docker.pkg.dev/<project>/stadera/
# under different image names (stadera-api, stadera-web).

resource "google_artifact_registry_repository" "stadera" {
  project       = var.project_id
  location      = var.region
  repository_id = local.ar_repo
  format        = "DOCKER"
  description   = "Container images for stadera-api and stadera-web."
}
