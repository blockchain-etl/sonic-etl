
environment           = "prod"
project_id            = "etl-services-447307"
project_id_short      = "sonic"
project_number        = "67685852234"
region                = "us-central1"
gke_node_machine_type = "n2-standard-2"

min_master_version = "1.31.5-gke.1023000"
nodepool_version   = "1.31.5-gke.1023000"

composer_version           = "composer-2.10.2-airflow-2.10.2"
composer_environment_names = ["mainnet-testnet"]

index_range_proto_name = "request"

create_default_network = true

enabled_networks = ["mainnet"]
gke_enabled      = true
dataflow_enabled = true

