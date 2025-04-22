// Either assign values to the local variables directly or define them in the variables
locals {
  project_id       = var.project_id
  project_id_short = var.project_id_short
  project_number   = var.project_number
  region           = var.region
  env              = var.environment
  default_labels = {
    env        = var.environment
    managed-by = "terraform"
  }

  index_topics = {
    transaction_indexing_ranges_mainnet = "indexing-ranges-mainnet"
    last_indexed_range_mainnet          = "last-indexed-range-mainnet"
  }

  schema_files     = fileset("../../schemas/bq", "*.json")
  schema_names     = [for file in local.schema_files : trimsuffix(basename(file), ".json")]
  all_schema_files = fileset("../../schemas", "**")

  dataset_names = [
    "crypto_sonic_mainnet_us",
  ]

  network_types = ["mainnet"]

  schemas_and_network_types = flatten([
    for schema in local.schema_names : [
      for network in local.network_types : {
        schema  = schema
        network = network
      }
    ]
  ])

  schemas_and_datasets = flatten([
    for dataset_name in local.dataset_names :
    [
      for schema_name in local.schema_names :
      {
        schema_file  = "${schema_name}.json"
        table_name   = schema_name
        dataset_name = dataset_name
      }
    ]
  ])

  dataflow_bucket = "gs://dataflow-templates-us-central1/latest/flex/PubSub_Proto_to_BigQuery_Flex"
}

variable "gke_master_ipv4_cidr_block" {
  type    = string
  default = "172.16.64.16/28"
}

variable "gke_node_machine_type" {
  type    = string
  default = "n2-standard-32"
}

variable "gke_node_disk_size_gb" {
  type    = number
  default = 200
}

variable "min_master_version" {
  type    = string
  default = "1.30.6-gke.1125000"
}

variable "nodepool_version" {
  type    = string
  default = "1.30.5-gke.1699000"
}

variable "composer_version" {
  type    = string
  default = "composer-2.10.1-airflow-2.10.2"
}

variable "composer_environment_names" {
  type = list(string)
}

variable "environment" {
  type = string
}

variable "project_id" {
  type = string
}

variable "project_id_short" {
  type = string
}

variable "project_number" {
  type = string
}

variable "region" {
  type = string
}

variable "index_range_proto_name" {
  type = string
}

variable "private_cidr_a" {
  type = map(string)
  default = {
    mainnet = "172.16.136.0/23"
    testnet = "172.16.138.0/23" # You can adjust testnet as needed
  }
}

variable "private_cidr_pods" {
  type = map(string)
  default = {
    mainnet = "10.108.0.0/14"
    testnet = "10.112.0.0/14" # You can adjust testnet as needed
  }
}

variable "private_cidr_services" {
  type = map(string)
  default = {
    mainnet = "10.84.96.0/20"
    testnet = "10.84.112.0/20" # You can adjust testnet as needed
  }
}

variable "create_default_network" {
  description = "Set to true to create the default network if it doesn't exist."
  type        = bool
  default     = false
}

variable "enabled_networks" {
  description = "Select networks to deploy (e.g., 'mainnet', 'testnet', or both)."
  type        = list(string)
  default     = ["mainnet"] # Default to mainnet, but you can select both if needed.
}

variable "gke_enabled" {
  description = "Flag to enable or disable GKE provisioning."
  type        = bool
  default     = true
}

variable "dataflow_enabled" {
  description = "Flag to enable or disable the ETL Dataflow jobs"
  type        = bool
  default     = true
}
