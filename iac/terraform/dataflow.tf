resource "google_dataflow_flex_template_job" "etl_dataflow_jobs" {
  for_each = var.dataflow_enabled ? { for idx, schema in local.schema_names : idx => schema } : {}

  provider                = google-beta
  name                    = "mainnet-${replace(each.value, "_", "-")}-job-prod"
  container_spec_gcs_path = local.dataflow_bucket
  service_account_email   = google_service_account.gcp_ingest_sa.email
  parameters = {
    outputTopic       = "projects/${local.project_id}/topics/errors-${each.value}-mainnet"
    protoSchemaPath   = "gs://${local.project_id}-sonic_schemas/pb/${each.value}.pb"
    inputSubscription = "projects/${local.project_id}/subscriptions/${replace(trimsuffix(each.value, "s"), "_", "-")}-records-mainnet-sub"
    outputTableSpec   = "${var.project_id}:crypto_sonic_mainnet_us.${each.value}"
    fullMessageName = "etl.${each.value}.${
      join("", [
        for part in split("_", trimsuffix(each.value, "s")) :
        "${upper(substr(part, 0, 1))}${substr(part, 1, -1)}"
      ])
    }"

    writeDisposition        = "WRITE_APPEND"
    createDisposition       = "CREATE_NEVER"
    bigQueryTableSchemaPath = "gs://${local.project_id}-sonic_schemas/bq/${each.value}.json"
    preserveProtoFieldNames = "true"
  }
  additional_experiments = ["streaming_mode_at_least_once"]
  depends_on = [
    google_pubsub_subscription.records_subs,
    google_storage_bucket_object.schema_objects,
    google_compute_network.default
  ]
}
