# Sonic ETL

This repo contains everything necessary to run an Sonic ETL pipeline (it is a monorepo).

## Quickstart
To deploy your own instance of this pipeline, you can find full end-to-end deployment scripts and documentation [here](/iac/).

## Architecture

![mainnet architecture](/assets/mainnet_streaming_architecture.png)

### Data Source

A Sonic mainnet archive node acts as the data source for this pipeline.

### Extraction and Transformation

1. Data extraction from the archive nodes and transformation into table records are performed by the Rust code in the `extractor_transformer/` directory. This code must be deployed with the required environment variables in Kubernetes. It also needs access to the Sonic node's gRPC port and must authenticate with GCP to dump the generated records into GCS buckets and subscribe to a Pub/Sub subscription.
2. To ensure that multiple instances of the extractor_transformer do not process the same data from the Sonic node, an *indexing coordinator* script must also be deployed in Kubernetes. This script, written in Python, is located in the `indexing_coordinator/` directory. It needs to authenticate with GCP to publish and subscribe to Pub/Sub topics.
3. The coordination performed by `indexing_coordinator` works by publishing ranges of transaction numbers (referred to as 'versions' on Sonic) to a Google Pub/Sub topic. The `extractor_transformer` instances pull their tasks from this Pub/Sub topic and make transaction requests to the node's gRPC interface in parallel. To ensure that the extractor_transformer instances do not receive duplicate messages from the Pub/Sub topic, all instances use the same `subscription` to the topic. This approach, known as 'competing consumers' or 'competing subscribers,' allows Pub/Sub to evenly distribute messages among subscribers during testing.

### Loading

Google Cloud Composer (Apache Airflow) is used to insert the records from the GCS buckets into BigQuery temporary tables. Then, Cloud Composer performs a SQL `MERGE` into the final BigQuery dataset to prevent duplicate records.

## Directories in this Repo

* `extractor_transformer`: Rust codebase for data extraction from the node, transformation into table records, and dumping into GCS buckets
* `indexing_coordinator`: Python codebase for coordinating multiple instances of `extractor_transformer` in Kubernetes
* `loader`: Cloud Composer scripts (aka Airflow DAGs) for loading data from GCS buckets into BigQuery
* `iac`: Infrastructure-as-code, such as terraform scripts, helm charts, and BigQuery tables and GCS buckets creation scripts
* `scripts`: Various utilities, such as build scripts for `extractor_transformer` and `indexing_coordinator`
* `schemas`: The table schemas for each of the BigQuery tables, in JSON format. Can be used to create the tables using `bq mk` command (also see `iac/create_tables.sh`)
