# Infrastructure as Code — Stadera with Terraform + Azure parallel

This document explains step by step what `terraform/` does, the IaC
concepts behind each piece, and how the same setup would translate to
Azure. The audience is someone who works day-to-day with Azure but is
new to GCP and wants to internalize the patterns rather than copy
commands.

## 1. Why IaC

Three problems IaC solves that ad-hoc `gcloud` / `az cli` don't:

| Problem | Without IaC | With IaC |
|---|---|---|
| **Reproducibility** | "I created the bucket via console last month, then ran 3 gcloud commands I don't remember" | `terraform apply` recreates it identically from versioned `.tf` files |
| **Drift detection** | someone edits a resource in console, you discover months later | `terraform plan` shows diff vs committed code on every run |
| **Change auditing** | git history shows code, but infra changes invisible | every infra change is a PR with `plan` output reviewable |
| **Disaster recovery** | "rebuild from memory" | `terraform apply` against new project rebuilds whole stack |
| **Onboarding** | new team member reads stale wiki, asks senior | `.tf` is the wiki, `terraform plan` is the verification |

The first three are valuable from day one. The last two scale with team
size — for solo Stadera, primarily (1) and (2).

## 2. The IaC tool we picked: Terraform

Alternatives we didn't pick and why:

| Tool | Why not for Stadera |
|---|---|
| **Pulumi** | uses real programming language (TS/Python/Go) → more flexible, but harder to reason about state vs declarative. Smaller ecosystem. |
| **CDK for Terraform** | thin wrapper over Terraform, adds abstraction layer. Worth it for very large IaC, overkill here. |
| **Native CDKs** (AWS CDK, Pulumi for Azure) | cloud-specific, locks you in. Terraform's HCL is portable across clouds. |
| **Crossplane** | Kubernetes-native IaC. Useful if you already run k8s, weird overhead if not. |
| **Native templates** (Deployment Manager / ARM / CloudFormation) | each cloud's first-party. Vendor lock-in + worse UX than Terraform. |

Terraform's killer feature for the day-to-day: **provider abstraction**.
Same workflow, mental model, and CLI for GCP, Azure, AWS, Cloudflare,
GitHub, Datadog, Stripe, … 100s of providers. Once you know Terraform,
you know IaC across all of them.

## 3. Core concepts (Terraform vocabulary)

### Provider

A plugin Terraform talks to to manage resources on a specific platform.
Stadera uses two:

```hcl
provider "google" {
  project = "<your-project-id>"
  region  = "europe-west1"
}

provider "google-beta" {
  project = "<your-project-id>"
  region  = "europe-west1"
}
```

`google` and `google-beta` are split because beta-only resources (e.g.
`google_cloud_run_domain_mapping`) need the `google-beta` provider
declared explicitly via `provider = google-beta` on the resource.

**Azure equivalent**: `provider "azurerm"` (Azure Resource Manager
provider). Same shape, different defaults:

```hcl
provider "azurerm" {
  features {}                          # required, can be empty
  subscription_id = "00000000-..."
  tenant_id       = "00000000-..."
}
```

There's also `provider "azuread"` for Entra ID resources. Same dual-
provider pattern as `google`/`google-beta`.

### Resource

A single managed entity. Each resource has a **type** (the GCP / Azure
object kind) and a **local name** (a Terraform-side identifier you'll
reference from other resources).

```hcl
resource "google_artifact_registry_repository" "stadera" {
  project       = var.project_id
  location      = var.region
  repository_id = "stadera"
  format        = "DOCKER"
}
```

- Type: `google_artifact_registry_repository`
- Local name: `stadera`
- Reference elsewhere: `google_artifact_registry_repository.stadera.id`

**Azure equivalent**: `resource "azurerm_container_registry" "stadera" { … }`.
Same syntax, different type names + slightly different attributes.

### State

Terraform's database of "what exists in the cloud, mapped to what's in
.tf". Lives in a file (`terraform.tfstate`). Critical:

- State is the **single source of truth** for what Terraform owns.
- If state and .tf disagree → `plan` shows changes.
- If state and reality disagree → `plan` shows changes (drift).
- Lose state → Terraform thinks nothing exists, would try to create
  duplicates.

**Local state** (default): `.tfstate` in current dir. Fine for solo,
horrible for teams (no locking, no shared truth).

**Remote state**: stored in cloud-managed backend with locking. Stadera
uses GCS:

```hcl
terraform {
  backend "gcs" {
    bucket = "stadera-tfstate"
    prefix = "prod"
  }
}
```

State is **sensitive**: it contains plaintext secrets, IP addresses,
SA keys, etc. Treat it like a credential. The GCS bucket should be
private (default) and versioned (so you can recover from bad applies).

**Azure equivalent**: `backend "azurerm"` storing in an Azure Storage
Account. Same locking semantics (via blob lease). Same sensitivity.

### Plan / Apply lifecycle

```
$ terraform plan
# Diffs current state vs .tf, prints "X to add, Y to change, Z to destroy"
# READ-ONLY. Safe to run anytime.

$ terraform apply
# Re-runs plan, then EXECUTES the diff. Mutates the cloud.
# Prompts for confirmation by default; --auto-approve to skip.
```

The discipline: never `apply` without reading the `plan` output. Even
"trivial" changes can mean unexpected destruction.

### Import

When resources already exist (created via console / CLI / clickops),
`terraform import` retroactively brings them into state without
recreating them.

```sh
terraform import google_artifact_registry_repository.stadera \
    projects/<your-project-id>/locations/europe-west1/repositories/stadera
```

After import, the resource is **in state** but the **.tf might not
match reality**. `terraform plan` shows the diff — you adjust .tf
until plan reports "No changes".

Stadera's PR-A is essentially a giant `import` operation. We declared
the .tf shape we expected, ran `terraform import` for each existing
resource, then chased plan-diffs to zero.

**Azure equivalent**: `terraform import azurerm_container_registry.acr /subscriptions/<sub-id>/resourceGroups/.../containerRegistries/...`. Same
mechanics, different ID format (Azure uses long resource IDs).

### Lifecycle blocks

Per-resource hints that change Terraform's update behavior:

```hcl
resource "google_cloud_run_v2_service" "api" {
  # ...
  lifecycle {
    ignore_changes = [template, client, client_version]
  }
}
```

This says: "after first apply, don't try to reconcile changes inside
`template`, `client`, `client_version`." Used in Stadera's
`cloud_run.tf` to keep CI's image-revision rollouts from fighting
Terraform's "shape" definition.

Other lifecycle directives:
- `prevent_destroy = true` — refuses to delete this resource even if
  someone removes it from .tf
- `create_before_destroy = true` — for blue-green replacement
- `replace_triggered_by = [...]` — recreate this resource when others change

## 4. Stadera's `terraform/` walkthrough

File by file, what each declares and why.

### `main.tf` — provider + backend

The entry point. Declares:
1. **`required_version`** → minimum Terraform CLI version the .tf has
   been tested with. Failsafe against syntax that doesn't exist in
   older versions.
2. **`required_providers`** → which providers this configuration uses,
   pinned to major versions. Pin loose enough to get bug fixes
   (`~> 6.0`), tight enough to catch breaking changes (`6.0` not `*`).
3. **`backend "gcs"`** → state storage location. CANNOT use variables
   here — the bucket must exist before init time.
4. **`provider`** blocks → per-provider config (project, region).

### `variables.tf` — input parameters

Variables let the same .tf instantiate multiple environments. For
Stadera (single env), we use them mostly for documentation: each
`variable` block names something parameterizable + describes it.

```hcl
variable "project_id" {
  description = "GCP project ID."
  type        = string
  default     = "<your-project-id>"
}
```

Override at apply time with `terraform apply -var project_id=other`,
or via `terraform.tfvars` file, or via `TF_VAR_project_id` env var.

### `locals.tf` — derived / computed values

Locals are like variables but computed inside the .tf, not provided
externally. Use them for:
- DRY (don't repeat the same string in 5 places)
- Naming conventions
- Computed values from data sources

```hcl
locals {
  service_name = "stadera-api"
  job_name     = "stadera-sync"
  pool_resource_name = "projects/${data.google_project.this.number}/locations/global/workloadIdentityPools/${...}"
}
```

### `service_accounts.tf` — non-human identities

Three SAs, each with a clear role:

- `stadera-api-deployer` — assumed by GitHub Actions on the backend
  repo, has project-level grants to roll Cloud Run revisions.
- `stadera-web-deployer` — same for the frontend repo.
- `stadera-sync-invoker` — used by Cloud Scheduler to invoke the
  daily sync Job.

Naming convention: `<service>-<purpose>`. `<purpose>` is one of:
- `-deployer` (CI deployment principal)
- `-invoker` (one resource calls another)
- `-runtime` (the SA the workload runs as) — **deferred to PR-B**

### `iam_bindings.tf` — who can do what

IAM in GCP and Azure shares the same fundamental model: **role
binding** = `(principal, role, scope)`. Stadera grants three roles
to each deployer:

| Role | Why |
|---|---|
| `roles/run.admin` | create / update / delete Cloud Run services + jobs |
| `roles/artifactregistry.writer` | push images |
| `roles/iam.serviceAccountUser` | "act as" the runtime SA when deploying |

The third one is sub-doc-of-its-own subtle. When you `gcloud run
services deploy --service-account=X`, Cloud Run associates the new
revision with SA X as runtime. To do that, the deployer needs
permission to *use* X — that permission is `iam.serviceAccountUser`.
Without it, deploy fails with "actAs denied".

### `artifact_registry.tf` — container registry

Trivial: one repo, format DOCKER, in `europe-west1`. Both backend and
frontend image streams push here under different names (`stadera-api`,
`stadera-web`).

### `wif.tf` — Workload Identity Federation

The pattern that lets GitHub Actions assume GCP SAs without storing
JSON keys:

1. **Pool** = container of trusted external identities for this project
2. **Provider** = how to verify external tokens; here OIDC from
   GitHub Actions
3. **principalSet binding** = "tokens matching this attribute can
   impersonate this SA"

The `attribute_condition = "assertion.repository=='..."` clause is the
security boundary — only OIDC tokens from a specific GitHub repo are
accepted. Without it, anyone with a GitHub token for any repo could
impersonate the deployer.

### `cloud_run.tf` — Service + Job + Domain Mappings

The actual workload. Two types:

- `google_cloud_run_v2_service` → long-running HTTP server
  (`stadera-api`). Auto-scales 0-2 instances based on request load.
  `min_instances=0` means cold starts but no idle cost.
- `google_cloud_run_v2_job` → batch job (`stadera-sync`). Runs the
  binary, exits, billing stops.

The `lifecycle.ignore_changes = [template, ...]` block on each is
critical: it makes Terraform leave the runtime config (image, env vars,
command, args) alone after first apply, so CI workflows can update
those without fighting Terraform.

Domain mappings (`api.stadera.org`, `app.stadera.org`) live here too.
They use the `google-beta` provider because the v1 stable provider
doesn't support them yet.

### `cloud_scheduler.tf` — daily cron

A `google_cloud_scheduler_job` that:
1. Wakes up at `0 6 * * *` Europe/Rome
2. POSTs to the Cloud Run Admin API to invoke the Job
3. Authenticates with an OIDC token minted as the `sync_invoker` SA
4. Cloud Run sees the token, verifies the SA has `run.invoker` on the
   Job, executes

The `oidc_token` block + `audience` are the keystone — without them
the call goes anonymous and Cloud Run returns 403.

### `outputs.tf` — exported values

Things humans (or scripts) read after apply. Stadera outputs:
- API URL (auto-generated *.run.app)
- Custom domain URLs
- WIF provider full path (paste into GitHub secret)
- Deployer SA emails (paste into GitHub secret)

`terraform output deployer_sa_backend` prints the value. Useful for
docs and for scripts that automate GitHub secret-setting.

## 5. The bootstrap chicken-and-egg

Terraform's GCS backend can't bootstrap itself. The bucket must exist
before `terraform init` can even configure the backend. Solutions
in order of complexity:

1. **Manual bootstrap** (Stadera's choice): create the bucket once with
   `gcloud storage buckets create`, then `terraform init` configures
   the backend. Documented in `terraform/README.md`.
2. **Bootstrap module**: separate Terraform configuration with local
   backend that creates the GCS bucket. Apply once, then move state
   to the bucket. More code, doesn't need gcloud.
3. **Workspace per stage**: one state for "platform" (bucket + IAM),
   another for "apps" using the bucket. Common in larger setups.

For Stadera (single env, single human), manual is simplest.

**Azure equivalent**: identical chicken-and-egg with Azure Storage
Account. Same three solutions.

## 6. The architecture call: shape vs revision

Stadera's CI workflow does TWO things:

1. **Build a new image** and push to Artifact Registry
2. **Roll a new Cloud Run revision** with that image

Terraform also "owns" the Cloud Run service. Conflict:

- If Terraform's .tf says `image = ":bootstrap"` and CI sets `image
  = ":sha-abc123"`, who wins?
- Without coordination, every `terraform apply` reverts to `:bootstrap`,
  every CI run reverts to `:sha-abc123`. War.

The split:

- **Terraform owns the SHAPE**: the resource exists, has these scaling
  bounds, this ingress, this CPU/memory, this IAM binding.
- **CI owns the REVISION**: the current image tag, env var values,
  command/args.

Implementation: `lifecycle.ignore_changes = [template, client, client_version]`.
Terraform stops tracking changes inside `template` after first
creation. CI updates `template` freely.

Result: changing `min_instance_count` is a Terraform PR. Changing the
running image is a code push. Both work, neither blocks the other.

## 7. Same architecture on Azure

Direct mapping of Stadera's resources:

| Stadera (GCP) | Azure equivalent |
|---|---|
| Cloud Run Service | **Azure Container Apps** (long-running) |
| Cloud Run Job | **Azure Container Apps Jobs** |
| Cloud Scheduler | **Logic Apps** with Recurrence trigger, OR Azure Functions Timer Trigger |
| Workload Identity Federation (GitHub → SA) | **Federated credentials** on App Registration / Managed Identity |
| Service Account | **Managed Identity** (preferred) or **App Registration** |
| Artifact Registry | **Azure Container Registry (ACR)** |
| Secret Manager | **Azure Key Vault** |
| Cloud Logging / Monitoring | **Azure Monitor Logs** (Log Analytics workspace) |
| GCS bucket (state) | **Azure Storage Account** blob container |
| Neon Postgres (managed) | **Azure Database for PostgreSQL Flexible Server**, OR keep external Neon |
| Cloud Run domain mapping | **Container Apps custom domain + cert binding** |
| GCS storage class / lifecycle | **Storage Account Lifecycle Management** |

Vocabulary translation that frequently trips people:

| GCP | Azure |
|---|---|
| Project | Subscription + Resource Group (Azure has 2 levels of grouping) |
| IAM principal | Service Principal (App Reg) or Managed Identity |
| IAM role | RBAC role (similar concept, very different role names) |
| Service Account key | Service Principal client secret OR client cert |
| Workload Identity Federation | Federated credentials on App Reg |
| Project number | Subscription ID + Tenant ID |
| Cloud Logging | Diagnostic Settings → Log Analytics |
| Cloud Build | Azure DevOps Pipelines / GitHub Actions |
| Cloud Storage bucket | Storage Account → Blob Container |
| Region | Region (different naming: `europe-west1` vs `westeurope`) |

### What Stadera's Terraform would look like on Azure

`main.tf`:

```hcl
terraform {
  required_providers {
    azurerm = { source = "hashicorp/azurerm", version = "~> 4.0" }
    azuread = { source = "hashicorp/azuread", version = "~> 3.0" }
  }
  backend "azurerm" {
    resource_group_name  = "stadera-tfstate-rg"
    storage_account_name = "staderatfstate"   # globally unique, alphanumeric
    container_name       = "tfstate"
    key                  = "prod.terraform.tfstate"
  }
}

provider "azurerm" {
  features {}
  subscription_id = var.subscription_id
}

provider "azuread" {}
```

`container_app.tf`:

```hcl
resource "azurerm_resource_group" "stadera" {
  name     = "stadera-rg"
  location = "westeurope"
}

resource "azurerm_log_analytics_workspace" "logs" {
  name                = "stadera-logs"
  location            = azurerm_resource_group.stadera.location
  resource_group_name = azurerm_resource_group.stadera.name
  sku                 = "PerGB2018"
  retention_in_days   = 30
}

resource "azurerm_container_app_environment" "stadera" {
  name                       = "stadera-env"
  location                   = azurerm_resource_group.stadera.location
  resource_group_name        = azurerm_resource_group.stadera.name
  log_analytics_workspace_id = azurerm_log_analytics_workspace.logs.id
}

resource "azurerm_container_app" "api" {
  name                         = "stadera-api"
  resource_group_name          = azurerm_resource_group.stadera.name
  container_app_environment_id = azurerm_container_app_environment.stadera.id
  revision_mode                = "Single"

  template {
    min_replicas = 0
    max_replicas = 2

    container {
      name   = "api"
      image  = "${azurerm_container_registry.stadera.login_server}/stadera-api:bootstrap"
      cpu    = 1
      memory = "2Gi"   # ACA min memory is higher than CR
    }
  }

  ingress {
    external_enabled = true
    target_port      = 8080
    traffic_weight {
      latest_revision = true
      percentage      = 100
    }
  }

  lifecycle {
    ignore_changes = [template, ingress, secret]
  }
}
```

Same pattern as Cloud Run: a `template` block holding the runtime
spec, `ignore_changes` to let CI roll revisions.

`identity.tf`:

```hcl
resource "azurerm_user_assigned_identity" "backend_deployer" {
  name                = "stadera-api-deployer"
  resource_group_name = azurerm_resource_group.stadera.name
  location            = azurerm_resource_group.stadera.location
}

# Federated credential: GitHub OIDC token → impersonate this identity
resource "azurerm_federated_identity_credential" "backend_deployer_github" {
  name                = "github-actions"
  resource_group_name = azurerm_resource_group.stadera.name
  parent_id           = azurerm_user_assigned_identity.backend_deployer.id
  audience            = ["api://AzureADTokenExchange"]
  issuer              = "https://token.actions.githubusercontent.com"
  subject             = "repo:MicheleBellitti/Stadera:ref:refs/heads/main"
}

resource "azurerm_role_assignment" "backend_deployer_acr" {
  scope                = azurerm_container_registry.stadera.id
  role_definition_name = "AcrPush"
  principal_id         = azurerm_user_assigned_identity.backend_deployer.principal_id
}

resource "azurerm_role_assignment" "backend_deployer_aca" {
  scope                = azurerm_resource_group.stadera.id
  role_definition_name = "Container Apps Contributor"
  principal_id         = azurerm_user_assigned_identity.backend_deployer.principal_id
}
```

The `subject` in the federated credential is the Azure equivalent of
GCP's `attribute.repository` condition. Same security boundary,
different syntax.

### Cron job on Azure

Container Apps Jobs got cron triggers in 2024 (previously you had to
use Logic Apps or Functions). Modern equivalent of Stadera's
Cloud Scheduler + Cloud Run Job:

```hcl
resource "azurerm_container_app_job" "sync" {
  name                         = "stadera-sync"
  location                     = azurerm_resource_group.stadera.location
  resource_group_name          = azurerm_resource_group.stadera.name
  container_app_environment_id = azurerm_container_app_environment.stadera.id

  replica_timeout_in_seconds   = 300
  replica_retry_limit          = 2

  schedule_trigger_config {
    cron_expression = "0 6 * * *"
    parallelism     = 1
    replica_completion_count = 1
  }

  template {
    container {
      name    = "sync"
      image   = "${azurerm_container_registry.stadera.login_server}/stadera-api:bootstrap"
      command = ["/usr/local/bin/stadera-jobs"]
      args    = ["sync", "--user-email", var.sync_user_email]
    }
  }
}
```

Note: this combines what GCP splits into "Cloud Run Job + Cloud
Scheduler" into a single Azure resource. Slightly cleaner.

### What's structurally different

A few places where Azure forces or rewards a different mental model:

1. **Resource Group** is mandatory. Every resource must be in one.
   In GCP, projects play that role but there's no intra-project
   grouping. Azure RGs are useful for: cost tagging, RBAC scope,
   bulk delete.
2. **Region naming**. `europe-west1` (GCP) ≠ `westeurope` (Azure).
   No translation table — manually map.
3. **No "service account" abstraction**. Azure has *App
   Registrations* (multi-tenant identities, you create in Entra ID)
   and *Managed Identities* (single-resource identities, auto-managed
   lifecycle). Use Managed Identity for "I'm a workload running in
   Azure". Use App Registration for "I'm a workload running outside
   Azure that needs to talk to Azure" (e.g. GitHub Actions before
   federation). Stadera's pattern (federated GitHub OIDC) uses
   Managed Identity + Federated Credential.
4. **Provider auth in `provider`** block is awkward. Recommended:
   omit subscription/tenant from .tf, let `az login` auth flow handle
   it via env. For CI, use OIDC federation.
5. **Azure RBAC role names are wordy**. `"Container Apps Contributor"`
   vs GCP's `"roles/run.admin"`. Same idea, more typing.

### What's the same

The patterns translate 1:1:

- IaC discipline (plan before apply)
- State stored remotely with locking
- Lifecycle blocks for shape-vs-revision split
- Federated credentials for keyless CI auth
- Resource imports for greenfield IaC over hand-managed infra

A team competent in Terraform-on-GCP picks up Terraform-on-Azure in
a few days. The 80% you already know is the same.

## 8. Common pitfalls

The ones to actively guard against:

### Drift accumulation

Someone (you, last week) changes a resource via console "for a quick
test" and forgets. `terraform plan` later shows the diff and you
either:
- Update .tf to match (acknowledging the change)
- `terraform apply` to revert (undoing the change)

Worst case: you `apply` without reading plan and silently revert
critical change. Mitigation: **discipline + small, frequent applies**.
The longer between applies, the more drift accumulates.

### State locking

Two people running `terraform apply` simultaneously corrupts state.
GCS / Azure Storage Account backends auto-lock during apply. If a
process dies mid-apply, the lock can stay stuck. `terraform force-unlock <id>`
releases it (only if you're sure no-one else is applying).

### Accidental destroy

`terraform plan` says "1 to destroy" and you apply without reading.
Especially dangerous for storage / databases / DNS records.
Mitigation:
- `lifecycle { prevent_destroy = true }` on critical resources
- Read every plan output before apply
- Use `-target=...` to limit applies to specific resources

### Hardcoded secrets in .tf

Never. State stores them in plaintext, .tf is committed to git.
Mitigation:
- Variables marked `sensitive = true` (Terraform redacts in plan/output)
- Source secrets from Secret Manager / Key Vault at apply time
- Or pass via env (`TF_VAR_secret=...`)

### Provider version drift

Your `terraform.lock.hcl` pins exact versions; .tf says `~> 6.0`. If
a teammate runs without lock file, they get a different provider
version and unpredictable behavior. Always commit the lock file.

### Module abstraction too early

Splitting into modules feels like good engineering. Until you have
2-3 instances of the "thing" you're abstracting, modules are pure
overhead — extra indirection, extra code, harder debugging. Stadera
explicitly stays single-file-per-concern (no modules) until we have
a real need (e.g. multiple environments).

## 9. Recovery scenarios

### "I broke prod, where's the backup"

State is versioned (GCS bucket has Object Versioning enabled). Restore
a previous state file:

```sh
# List versions
gcloud storage ls -a gs://stadera-tfstate/prod.tfstate
# Restore a specific generation
gcloud storage cp gs://stadera-tfstate/prod.tfstate#1234567890 \
    gs://stadera-tfstate/prod.tfstate
```

### "Terraform thinks resource exists but I deleted via console"

Delete from state to acknowledge:

```sh
terraform state rm google_cloud_run_v2_service.api
```

Next `apply` will recreate (or you remove from .tf entirely).

### "Need to migrate state to a new backend"

```sh
# Edit main.tf to point at new backend
terraform init -migrate-state
# Terraform copies state to new backend interactively
```

## 10. Where to learn more

- [HashiCorp Terraform docs](https://developer.hashicorp.com/terraform) — reference for HCL, CLI, providers
- [Google Cloud Terraform reference](https://registry.terraform.io/providers/hashicorp/google/latest/docs)
- [Azure Terraform reference](https://registry.terraform.io/providers/hashicorp/azurerm/latest/docs)
- [Gruntwork's Terraform book](https://www.terraformupandrunning.com/) — most-recommended deep dive
- [Azure-to-GCP cheatsheet (Google)](https://cloud.google.com/docs/get-started/aws-azure-gcp-service-comparison) — vocabulary translation
