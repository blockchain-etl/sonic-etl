locals {
  proto_dir     = abspath("${path.module}/../../schemas/proto")
  pb_dir        = abspath("${path.module}/../../schemas/pb")
  pb_files_hash = fileexists("${local.pb_dir}/hash.txt") ? filemd5("${local.pb_dir}/hash.txt") : timestamp()
}
resource "google_storage_bucket" "composer_dag_bucket" {
  for_each      = toset(var.enabled_networks)
  name          = "${local.project_id_short}-composer-dag-bucket-${each.value}"
  location      = local.region
  force_destroy = true
}

resource "google_storage_bucket" "fullnode_backups" {
  for_each      = toset(["mainnet", "testnet"])
  name          = "${local.project_id_short}-${local.env}-${each.key}-backups"
  location      = local.region
  storage_class = "STANDARD"

  force_destroy = false

  versioning { enabled = true }

  lifecycle_rule {
    condition { num_newer_versions = 5 }
    action { type = "Delete" }
  }

  labels = local.default_labels
}

resource "google_storage_bucket" "etl_buckets" {
  for_each      = { for name in local.schema_names : name => name }
  name          = "${local.project_id}-${var.project_id_short}_testnet_${each.key}"
  location      = local.region
  storage_class = "STANDARD"
  force_destroy = true
}

resource "google_storage_bucket" "etl_schema_bucket" {
  name          = "${local.project_id}-${var.project_id_short}_schemas"
  location      = local.region
  storage_class = "STANDARD"
  force_destroy = true
}

resource "null_resource" "generate_pb_files" {
  triggers = {
    always_run = "${timestamp()}" # Forces Terraform to run the command each time
  }
  provisioner "local-exec" {
    command     = <<EOT
      mkdir -p ${local.pb_dir}
      echo "Generating .pb files..."
      for schema in ${join(" ", local.schema_names)}; do
        echo "Processing schema: $${schema}"
        protoc --proto_path=${local.proto_dir} \
               --descriptor_set_out=${local.pb_dir}/$${schema}.pb \
               $${schema}.proto
        echo "Generated: ${local.pb_dir}/$${schema}.pb"
      done
      echo "All .pb files updated."
    EOT
    working_dir = local.proto_dir
  }
}

resource "google_storage_bucket_object" "schema_objects" {
  for_each = { for file in local.all_schema_files : file => file }

  name           = trimprefix(each.value, "../../schemas/")
  bucket         = "${local.project_id}-${var.project_id_short}_schemas"
  source         = "../../schemas/${each.value}"
  detect_md5hash = true
  depends_on = [
    google_storage_bucket.etl_schema_bucket,
    null_resource.generate_pb_files
  ]
}
