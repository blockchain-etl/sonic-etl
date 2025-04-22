# Service Accounts
resource "google_service_account" "k8s_cluster" {
  count        = var.gke_enabled ? 1 : 0
  account_id   = "${local.project_id_short}-k8s"
  display_name = "${local.project_id_short}-k8s"
}

resource "google_service_account" "k8s_storage" {
  count        = var.gke_enabled ? 1 : 0
  account_id   = "${local.project_id_short}-k8s-storage"
  display_name = "${local.project_id_short}-k8s-storage"
}

resource "google_service_account" "etl_app_sa" {
  count        = var.gke_enabled ? 1 : 0
  account_id   = "${local.project_id_short}-etl-app"
  display_name = "${local.project_id_short}-etl-app"
}

resource "google_service_account" "gcp_ingest_sa" {
  account_id   = "etl-gcp-ingest-sa"
  display_name = "etl-gcp-ingest-sa"
}

resource "google_service_account" "etl_ci" {
  account_id   = "etl-ci"
  display_name = "ETL CI Service Account"
}


# IAM Roles and Workload Identity Binding
locals {
  k8s_cluster_iam_roles = [
    "roles/logging.logWriter",
    "roles/monitoring.metricWriter",
    "roles/monitoring.viewer",
    "roles/stackdriver.resourceMetadata.writer",
    "roles/storage.objectViewer",
    "roles/artifactregistry.reader",
  ]

  k8s_storage_iam_roles = [
    "roles/storage.objectViewer",
    "roles/storage.objectAdmin",
    "roles/storage.admin",
  ]

  etl_app_iam_roles = [
    "roles/pubsub.publisher",
    "roles/pubsub.subscriber",
    "roles/pubsub.viewer",
    "roles/storage.admin",
    "roles/monitoring.viewer",
    "roles/monitoring.metricWriter",
    "roles/storage.objectCreator",
    "roles/artifactregistry.reader",
  ]

  gcp_ingest_iam_roles = [
    "roles/bigquery.dataOwner",
    "roles/bigquery.jobUser",
    "roles/composer.worker",
    "roles/dataflow.worker",
  ]

  etl_ci_iam_roles = [
    "roles/container.admin",
    "roles/container.clusterViewer",
    "roles/container.developer",
    "roles/dataflow.admin",
    "roles/dataflow.worker",
    "roles/monitoring.viewer",
    "roles/pubsub.admin",
    "roles/pubsub.viewer",
    "roles/secretmanager.secretAccessor",
    "roles/storage.admin",
    "roles/storage.objectAdmin",
    "roles/storage.objectViewer",
    "roles/viewer",
    "roles/artifactregistry.writer"
  ]
}

# IAM Members for K8s Cluster
resource "google_project_iam_member" "k8s_cluster" {
  for_each = var.gke_enabled ? toset(local.k8s_cluster_iam_roles) : toset([])
  project  = local.project_id
  role     = each.key
  member   = "serviceAccount:${google_service_account.k8s_cluster[0].email}"
}

# IAM Members for K8s Storage
resource "google_project_iam_member" "k8s_storage" {
  for_each = var.gke_enabled ? toset(local.k8s_storage_iam_roles) : toset([])
  project  = local.project_id
  role     = each.key
  member   = "serviceAccount:${google_service_account.k8s_storage[0].email}"
}


# IAM Members for ETL App Service Account
resource "google_project_iam_member" "etl_app_sa" {
  for_each = var.gke_enabled ? toset(local.etl_app_iam_roles) : toset([])
  project  = local.project_id
  role     = each.key
  member   = "serviceAccount:${google_service_account.etl_app_sa[0].email}"
}

resource "google_project_iam_member" "etl_ci" {
  for_each = toset(local.etl_ci_iam_roles)
  project  = local.project_id
  role     = each.key
  member   = "serviceAccount:${google_service_account.etl_ci.email}"
}


# Workload Identity Binding for ETL App Service Account
resource "google_service_account_iam_binding" "etl_app_sa" {
  count              = var.gke_enabled ? 1 : 0
  service_account_id = google_service_account.etl_app_sa[0].name
  role               = "roles/iam.workloadIdentityUser"
  members = [
    "serviceAccount:${local.project_id}.svc.id.goog[mainnet/etl-app-sa]",
    "serviceAccount:${local.project_id}.svc.id.goog[testnet/etl-app-sa]",
    "serviceAccount:${local.project_id}.svc.id.goog[keda/keda-operator]"
  ]
}

# IAM Members for GCP Ingest Service Account
resource "google_project_iam_member" "gcp_ingest_sa" {
  for_each = toset(local.gcp_ingest_iam_roles)
  project  = local.project_id
  role     = each.key
  member   = "serviceAccount:${google_service_account.gcp_ingest_sa.email}"
}

resource "google_service_account_iam_member" "gcp_ingest_sa" {
  provider           = google-beta
  service_account_id = google_service_account.gcp_ingest_sa.name
  role               = "roles/composer.ServiceAgentV2Ext"
  member             = "serviceAccount:service-${local.project_number}@cloudcomposer-accounts.iam.gserviceaccount.com"
}

# Composer Service Agent Role
resource "google_project_iam_member" "composer_service_agent_role" {
  project = local.project_id
  role    = "roles/composer.ServiceAgentV2Ext"
  member  = "serviceAccount:${local.project_number}-compute@developer.gserviceaccount.com"
}
