locals {
  def_files = [for name in local.schema_names : "${name}.proto"]
}

data "local_file" "proto_defs" {
  for_each = { for file in local.def_files : file => file }
  filename = "../../schemas/proto/${each.key}"
}

data "local_file" "index_range_proto_def" {
  filename = "../../schemas/proto/${var.index_range_proto_name}.proto"
}

resource "google_pubsub_schema" "pubsub_schemas" {
  for_each   = { for file, _ in data.local_file.proto_defs : file => file }
  name       = trimsuffix(each.value, ".proto")
  type       = "PROTOCOL_BUFFER"
  definition = data.local_file.proto_defs[each.key].content
}

resource "google_pubsub_schema" "index_range_schema" {
  name       = "indexing-range-pb"
  type       = "PROTOCOL_BUFFER"
  definition = data.local_file.index_range_proto_def.content
}

resource "google_pubsub_topic" "records_topics" {
  for_each = {
    for item in local.schemas_and_network_types :
    "${item.schema}-${item.network}" => item
  }

  name = "${replace(trimsuffix(each.value.schema, "s"), "_", "-")}-records-${each.value.network}"

  depends_on = [google_pubsub_schema.pubsub_schemas]
  schema_settings {
    schema   = "projects/${local.project_id}/schemas/${each.value.schema}"
    encoding = "BINARY"
  }
}

resource "google_pubsub_topic" "errors_topics" {
  for_each = {
    for item in local.schemas_and_network_types :
    "${item.schema}-${item.network}" => item
  }

  name                       = "errors-${each.value.schema}-${each.value.network}"
  message_retention_duration = "2678400s"
}

resource "google_pubsub_topic" "transaction_index_topics" {
  for_each = {
    for topic in local.index_topics :
    topic => topic
  }
  name = each.value
  depends_on = [
    google_pubsub_schema.pubsub_schemas,
    google_pubsub_schema.index_range_schema
  ]
  schema_settings {
    schema   = "projects/${local.project_id}/schemas/indexing-range-pb"
    encoding = "BINARY"
  }
}

resource "google_pubsub_subscription" "records_subs" {
  for_each = {
    for item in local.schemas_and_network_types :
    "${replace(trimsuffix(item.schema, "s"), "_", "-")}-${item.network}" => item
  }

  name                       = "${replace(trimsuffix(each.value.schema, "s"), "_", "-")}-records-${each.value.network}-sub"
  topic                      = "projects/${local.project_id}/topics/${replace(trimsuffix(each.value.schema, "s"), "_", "-")}-records-${each.value.network}"
  message_retention_duration = "604800s"
  ack_deadline_seconds       = 10

  expiration_policy {
    ttl = "2678400s"
  }

  depends_on = [
    google_pubsub_topic.records_topics
  ]
}

resource "google_pubsub_subscription" "transaction_index_subs" {
  for_each = {
    for topic in local.index_topics :
    topic => topic
  }
  name                       = "${each.value}-sub"
  topic                      = "projects/${local.project_id}/topics/${each.value}"
  message_retention_duration = "604800s"
  ack_deadline_seconds       = 600
  expiration_policy {
    ttl = "2678400s"
  }

  depends_on = [google_pubsub_topic.transaction_index_topics]
}

resource "google_pubsub_subscription" "indexing_ranges_testnet" {
  count = contains(var.enabled_networks, "testnet") ? 1 : 0

  name                       = "indexing-ranges-subscription-testnet"
  topic                      = "projects/${var.project_id}/topics/indexing-ranges-testnet"
  message_retention_duration = "604800s"
  ack_deadline_seconds       = 10

  expiration_policy {
    ttl = "2678400s"
  }

  depends_on = [
    google_pubsub_topic.transaction_index_topics,
  ]
}

resource "google_pubsub_topic" "etl_bq" {
  for_each = { for env in var.enabled_networks : env => env }
  name     = "${local.project_id}-${each.key}"
  labels   = local.default_labels
}

resource "google_pubsub_subscription" "errors_subs" {
  for_each = {
    for item in local.schemas_and_network_types :
    "${item.network}-${item.schema}" => item
  }

  name                       = "${each.value.network}-${each.value.schema}-errors-subscription-gchat"
  topic                      = "projects/${local.project_id}/topics/errors-${each.value.schema}-${each.value.network}"
  message_retention_duration = "604800s"
  ack_deadline_seconds       = 10

  depends_on = [
    google_pubsub_topic.errors_topics
  ]
}

