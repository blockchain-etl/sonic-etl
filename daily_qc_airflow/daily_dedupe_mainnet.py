from airflow.models import Variable
PROJECT = f"etl-services-447307"
NETWORK_TYPE = "mainnet" # TODO: environment variable?
FINAL_DATASET = f"crypto_sonic_{NETWORK_TYPE}_us"

import json
from datetime import datetime, timedelta
from airflow.models import DAG
from airflow.utils.task_group import TaskGroup
from airflow.operators.bash import BashOperator
from airflow.operators.python import PythonOperator
from airflow.operators.empty import EmptyOperator
from airflow.providers.google.cloud.operators.bigquery import (
    BigQueryCreateEmptyDatasetOperator, BigQueryCreateEmptyTableOperator
)
from airflow.providers.google.cloud.transfers.gcs_to_local import GCSToLocalFilesystemOperator
from airflow.providers.google.cloud.operators.bigquery import BigQueryExecuteQueryOperator
from google_chat_callbacks import task_fail_alert

TODATE = "{{ (logical_date + macros.timedelta(days=1)).strftime('%Y-%m-%d') }}"
YESTERDATE = "{{ logical_date.strftime('%Y-%m-%d') }}"

tables = ["blocks", "decoded_events", "logs", "receipts", "traces", "transactions"]


# the fields for each table that determines if two records are unique or duplicates
# NOTE: for 4/8 tables, it's just `tx_version` and `change_index`
SQL_MERGE_TABLE = {
    "blocks": "block_number",
    "decoded_events": "block_number, transaction_index, log_index",
    "logs": "block_number, transaction_index, log_index",
    "receipts": "block_number, transaction_index",
    "traces": "block_number, transaction_index, trace_index",
    "transactions": "block_number, transaction_index"
}


default_args = {
    "depends_on_past": False,
    'start_date': datetime(2024, 12, 1),
    "provide_context": True,
    "retries": 4,
    "retry_delay": timedelta(minutes=1),
    #"on_success_callback": task_success_alert,
    "on_failure_callback": task_fail_alert,
}


# runs 2 hours after midnight, every day
# NOTE: this needs to be 90 minutes after we finish streaming this data, because BQ doesn't let us UPDATE or DELETE records until 90 minutes after streaming
# BONUS NOTE: the daily backfill QC runs 30 minutes after midnight, so 90 minutes after that is 2 hours past midnight.
with DAG(
    dag_id="daily_dedupe_mainnet",
    schedule_interval="0 2 * * *",
    default_args=default_args,
    catchup=True,
) as dag:

    start_task = EmptyOperator(task_id="start_task")

    with TaskGroup("bq_dedupe_task") as bq_dedupe_task:
        for table_name in tables:
            with TaskGroup(f"gcs_to_bq_{table_name}") as gcs_to_bq_tasks:
                # Add Data to BigQuery Public Dataset
                dedupe_sql = f"""
MERGE INTO `{PROJECT}.{FINAL_DATASET}.{table_name}` AS INTERNAL_DEST
USING (
SELECT k.*
FROM (
SELECT ARRAY_AGG(original_data LIMIT 1)[OFFSET(0)] k
FROM `{PROJECT}.{FINAL_DATASET}.{table_name}` AS original_data
WHERE block_timestamp BETWEEN TIMESTAMP('{YESTERDATE}') AND TIMESTAMP('{TODATE}')
GROUP BY {SQL_MERGE_TABLE[table_name]}
)
) AS INTERNAL_SOURCE
ON FALSE
WHEN NOT MATCHED BY SOURCE
AND INTERNAL_DEST.block_timestamp BETWEEN TIMESTAMP('{YESTERDATE}') AND TIMESTAMP('{TODATE}')
THEN DELETE
WHEN NOT MATCHED THEN INSERT ROW;
                """

                bq_dedupe_task_inner = BigQueryExecuteQueryOperator(
                    task_id=f"{table_name}_dedupe_bq_task",
                    #destination_dataset_table=f"{FINAL_PROJECT}.{FINAL_DATASET}.{table_name}",
                    sql=dedupe_sql,
                    use_legacy_sql=False,
                    create_disposition="CREATE_NEVER",
                    #write_disposition="WRITE_DISPOSITION_UNSPECIFIED",
                    #fields=["friendlyName", "description"],
                    #schema_object=f"{table_name}.json",
                )

            bq_dedupe_task_inner


    end_task = EmptyOperator(task_id="end_task")

    (
        start_task
        >> bq_dedupe_task
        >> end_task
    )
