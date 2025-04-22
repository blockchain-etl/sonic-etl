# Local values for Kubernetes node locations and tags
locals {
  # If gke_enabled is false, provide an empty map to avoid errors
  k8s_node_locations = { for env in var.enabled_networks : env => ["${local.region}-c"] if var.gke_enabled }
  k8s_node_tag       = { for env in var.enabled_networks : env => "${local.project_id}-${env}-k8s-node" if var.gke_enabled }
}

# Google Kubernetes Engine Cluster
resource "google_container_cluster" "etl_bq" {
  for_each                    = var.gke_enabled ? { for env in var.enabled_networks : env => env } : {}
  provider                    = google-beta
  name                        = "${local.project_id}-${each.key}"
  location                    = local.region
  network                     = google_compute_network.etl[each.key].self_link
  subnetwork                  = google_compute_subnetwork.etl_k8s[each.key].self_link
  networking_mode             = "VPC_NATIVE"
  enable_intranode_visibility = true
  datapath_provider           = "ADVANCED_DATAPATH"
  min_master_version          = var.min_master_version

  node_locations           = local.k8s_node_locations[each.key]
  initial_node_count       = 1
  remove_default_node_pool = true
  deletion_protection      = false

  release_channel { channel = "REGULAR" }
  binary_authorization { evaluation_mode = "DISABLED" }
  cluster_autoscaling { autoscaling_profile = "OPTIMIZE_UTILIZATION" }

  addons_config {
    gce_persistent_disk_csi_driver_config { enabled = true }
    gke_backup_agent_config { enabled = true }
  }

  ip_allocation_policy {
    cluster_secondary_range_name  = "${local.project_id}-${each.key}-k8s-pods"
    services_secondary_range_name = "${local.project_id}-${each.key}-k8s-services"
  }

  private_cluster_config {
    enable_private_nodes    = true
    enable_private_endpoint = false
    master_ipv4_cidr_block  = var.gke_master_ipv4_cidr_block
  }

  workload_identity_config {
    workload_pool = "${local.project_id}.svc.id.goog"
  }

  lifecycle {
    ignore_changes = [node_locations]
  }


  logging_config {
    enable_components = ["SYSTEM_COMPONENTS"]
  }

  monitoring_config {
    managed_prometheus { enabled = false }
  }

  notification_config {
    pubsub {
      enabled = true
      topic   = google_pubsub_topic.etl_bq[each.key].id
    }
  }

  node_config {
    service_account = google_service_account.k8s_cluster[0].email
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]
  }

  resource_labels = local.default_labels
}

# Google Container Node Pool for etl
resource "google_container_node_pool" "etl" {
  for_each       = var.gke_enabled ? { for env in var.enabled_networks : env => env } : {}
  name           = "etl-${each.key}"
  cluster        = google_container_cluster.etl_bq[each.key].name
  location       = local.region
  node_locations = local.k8s_node_locations[each.key]

  initial_node_count = 1

  autoscaling {
    location_policy      = "ANY"
    total_min_node_count = 1
    total_max_node_count = 10
  }

  lifecycle {
    ignore_changes = [
      initial_node_count,
      node_config,
      node_locations,
      management
    ]
  }

  management {
    auto_repair  = true
    auto_upgrade = true
  }

  upgrade_settings {
    max_surge       = 1
    max_unavailable = 0
  }

  node_config {
    preemptible  = false
    machine_type = var.gke_node_machine_type
    image_type   = "COS_CONTAINERD"
    disk_size_gb = var.gke_node_disk_size_gb

    workload_metadata_config { mode = "GKE_METADATA" }

    shielded_instance_config {
      enable_secure_boot          = true
      enable_integrity_monitoring = true
    }

    service_account = google_service_account.k8s_cluster[0].email
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]

    resource_labels = local.default_labels

    tags = [local.k8s_node_tag[each.key]]

    taint {
      key    = "usage"
      value  = "etl-${each.key}"
      effect = "NO_SCHEDULE"
    }
  }
}

# Google Container Node Pool for etl_apps
resource "google_container_node_pool" "etl_apps" {
  for_each       = var.gke_enabled ? { for env in var.enabled_networks : env => env } : {}
  name           = "etl-apps-${each.key}"
  cluster        = google_container_cluster.etl_bq[each.key].name
  location       = local.region
  node_locations = local.k8s_node_locations[each.key]

  initial_node_count = 1

  autoscaling {
    location_policy      = "ANY"
    total_min_node_count = 1
    total_max_node_count = 10
  }

  lifecycle {
    ignore_changes = [
      initial_node_count,
      node_config,
      node_locations,
      management
    ]
  }

  management {
    auto_repair  = true
    auto_upgrade = true
  }

  upgrade_settings {
    max_surge       = 1
    max_unavailable = 0
  }

  node_config {
    preemptible  = false
    machine_type = var.gke_node_machine_type
    image_type   = "COS_CONTAINERD"
    disk_size_gb = var.gke_node_disk_size_gb

    workload_metadata_config { mode = "GKE_METADATA" }

    shielded_instance_config {
      enable_secure_boot          = true
      enable_integrity_monitoring = true
    }

    service_account = google_service_account.k8s_cluster[0].email
    oauth_scopes    = ["https://www.googleapis.com/auth/cloud-platform"]

    resource_labels = local.default_labels

    tags = [local.k8s_node_tag[each.key]]
  }
}
