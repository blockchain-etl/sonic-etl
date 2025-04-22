from airflow import DAG
from airflow.utils.task_group import TaskGroup
from airflow.exceptions import AirflowException
from airflow.operators.python_operator import PythonOperator
from airflow.providers.google.cloud.hooks.bigquery import BigQueryHook
from datetime import datetime
from airflow.operators.empty import EmptyOperator
import logging
from google_chat_callbacks import task_fail_alert

FINAL_PROJECT = "ETL Services"
NETWORK_TYPE = "mainnet"
FINAL_DATASET = f"crypto_sonic_{NETWORK_TYPE}_us"

with DAG(
    'hourly_system_check',
    default_args={
        'owner': 'airflow',
        'depends_on_past': False,
        'start_date': datetime(2024, 12, 8),
        'on_failure_callback': task_fail_alert,
        'email_on_failure': False,
        'email_on_retry': False,
        'retries': 0,
    },
    description='Hourly system check QC (mainnet)',
    schedule_interval="0 * * * *",
    max_active_runs=1,
    catchup=False,
) as dag:
    start_task = EmptyOperator(task_id="start_task")

    def bigquery_all_versions_f(last_hour, this_hour):
        sql = f"""
        SELECT block_number
        FROM `{FINAL_PROJECT}.{FINAL_DATASET}.blocks`
        WHERE block_timestamp BETWEEN TIMESTAMP('{last_hour}') AND TIMESTAMP('{this_hour}') ORDER BY block_number
        """
        hook = BigQueryHook(use_legacy_sql=False)
        result = hook.get_pandas_df(sql=sql)
        result_as_list = result['block_number'].tolist()

        if len(result_as_list) == 0:
            raise AirflowException("No blocks in the past hour")
        else:
            return

    bigquery_all_versions = PythonOperator(
        task_id="bigquery_all_versions",
        python_callable=bigquery_all_versions_f,
        op_args=['{{ logical_date.strftime("%Y-%m-%d") }}', '{{ (logical_date + macros.timedelta(hours=1)).strftime("%Y-%m-%d") }}'],
        do_xcom_push=True,
        provide_context=True,
        dag=dag,
    )

    end_task = EmptyOperator(task_id="end_task")

    # Set task dependencies
    (
        start_task >> bigquery_all_versions >> end_task
    )
