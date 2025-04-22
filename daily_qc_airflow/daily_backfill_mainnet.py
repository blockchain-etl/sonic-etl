
from airflow import DAG
from airflow.operators.python_operator import PythonOperator
from airflow.providers.google.cloud.hooks.bigquery import BigQueryHook
from airflow.providers.google.cloud.hooks.pubsub import PubSubHook
from airflow.models import Variable
from datetime import timedelta, datetime
from airflow.operators.empty import EmptyOperator
from airflow.decorators import dag, task
import logging

from pubsub_range_pb2 import IndexingRequest
from google_chat_callbacks import task_fail_alert

PROJECT = f"etl-services-447307"
NETWORK_TYPE = "mainnet" # TODO: environment variable?
FINAL_DATASET = f"crypto_sonic_{NETWORK_TYPE}_us"
INDEXING_RANGES_TOPIC_NAME = f"indexing-ranges-{NETWORK_TYPE}"
MAX_RECORD_CNT = 1000;

TODATE = "{{ (logical_date + macros.timedelta(days=1)).strftime('%Y-%m-%d') }}"
YESTERDATE = "{{ logical_date.strftime('%Y-%m-%d') }}"

# runs 30 minutes after midnight:
# 1. want to ensure that streaming has finished
# 2. don't need to wait 90 minutes for the streaming buffer to finish

with DAG(
    'backfill_missing_blocks_mainnet_prod',
    default_args={
        'owner': 'airflow',
        'depends_on_past': False,
        'start_date': datetime(2024, 12, 1),
        "on_failure_callback": task_fail_alert,
        'email_on_failure': False,
        'email_on_retry': False,
        'retries': 1,
        'retry_delay': timedelta(minutes=1),
    },
    description='Daily backfill QC (mainnet)',
    schedule_interval="30 0 * * *",
    max_active_runs=1,
    catchup=True,
) as dag:
    start_task = EmptyOperator(task_id="start_task")

    def bigquery_all_blocknumbers_f(yesterdate, todate):
        sql = f"""
        SELECT block_number
        FROM `{PROJECT}.{FINAL_DATASET}.blocks`
        WHERE block_timestamp BETWEEN TIMESTAMP('{yesterdate}') AND TIMESTAMP('{todate}') ORDER BY block_number
        """
        hook = BigQueryHook(use_legacy_sql=False)
        result = hook.get_pandas_df(sql=sql)
        result_as_list = result['block_number'].tolist()
        return result_as_list

    bigquery_all_blocknumbers = PythonOperator(
        task_id="bigquery_all_blocknumbers",
        python_callable=bigquery_all_blocknumbers_f,
        op_args=['{{ logical_date.strftime("%Y-%m-%d") }}', '{{ (logical_date + macros.timedelta(days=1)).strftime("%Y-%m-%d") }}'],
        do_xcom_push=True,
        provide_context=True,
        dag=dag,
    )

    # use the bigquery query results from the previous task to create a python list of all integers from min to max (inclusive)
    def generate_array(**context):
        all_versions = context['task_instance'].xcom_pull(task_ids='bigquery_all_blocknumbers')
        min_version = all_versions[0]
        max_version = all_versions[-1]

        return list(range(min_version, max_version + 1))

    # Task 2: Generate array of all integers from the daily [min, max]
    generate_integers_array = PythonOperator(
        task_id='generate_integers_array',
        python_callable=generate_array,
        provide_context=True,
        do_xcom_push=True,
        dag=dag,
    )

    def check_missing_values_f(**context):
        all_versions = context['task_instance'].xcom_pull(task_ids='bigquery_all_blocknumbers')
        all_versions_set = set(all_versions)

        all_integers = context['task_instance'].xcom_pull(task_ids='generate_integers_array')
        missing_versions = [n for n in all_integers if n not in all_versions_set]
        missing_versions.sort()
        return missing_versions

    check_missing_values = PythonOperator(
        task_id="check_missing_values",
        python_callable=check_missing_values_f,
        provide_context=True,
        do_xcom_push=True,
        dag=dag,
    )

    # create an IndexingRange protobuf object from the missing values calculated in previous task, then serialize it and publish to pub/sub
    def publish_range_f(**context):

        def chunk_range(start, end):

            # Make sure start is before or at end.  If the difference is less than
            # MAX_RECORD_CNT, simply yield that, otherwise we need to break things
            # into chunks.

            if start > end:
                raise ValueError(f"Start ({start}) cannot be > than End ({end})")
            elif (end - start) < MAX_RECORD_CNT:
                yield (start, end)
                return
            else:
                # Yield each chunk, where the start
                chunk_start = start
                while chunk_start <= end:
                    chunk_end = min(chunk_start + MAX_RECORD_CNT - 1, end)
                    yield (chunk_start, chunk_end)
                    chunk_start = chunk_end + 1
                return




        missing_values = context['task_instance'].xcom_pull(task_ids='check_missing_values')
        # this is allowed to fail due to no missing values
        try:
            ranges = []
            if len(missing_values) > 1:
                # for multiple possible ranges
                cur_start = missing_values[0]
                cur_end = missing_values[0]
                for n in missing_values[1:]:
                    if n > cur_end + 1:
                        ranges.append((cur_start, cur_end))
                        cur_start = n
                    cur_end = n
                else:
                    ranges.append((cur_start, cur_end))
            elif len(missing_values) == 1:
                # for single case
                ranges.append((missing_values[0], missing_values[0]))

            for (start, end) in ranges:
                logging.getLogger(__name__).info(f"(start, end) of ranges is ({start},{end})")
                # start, end = min(missing_values), max(missing_values)
                for (chunk_start, chunk_end) in chunk_range(start, end):
                    logging.getLogger(__name__).info(f"(chunk_start, chunk_end) of ranges is ({chunk_start},{chunk_end})")
                    indexing_range_proto_obj = IndexingRequest(start=chunk_start, end=chunk_end, blocks=True, logs=True, transactions=True, receipts=True, decoded_events=True, traces=True) # fun fact: positional arguments don't work with protobuf initializers
                    logging.getLogger(__name__).info(f"indexing range is: {indexing_range_proto_obj}")
                    assert(indexing_range_proto_obj is not None)
                    indexing_range_serialized = indexing_range_proto_obj.SerializeToString()
                    hook = PubSubHook(use_legacy_sql=False)
                    result = hook.publish(project_id=PROJECT, topic=INDEXING_RANGES_TOPIC_NAME, messages=[{"data": indexing_range_serialized}])
                    logging.getLogger(__name__).info(f"publish result is: {result}")
        finally:
            return

    # use the min and max values to publish a backfilling range
    publish_range = PythonOperator(
        task_id='publish_range',
        python_callable=publish_range_f,
        provide_context=True,
        do_xcom_push=True,
        dag=dag,
    )

    end_task = EmptyOperator(task_id="end_task")

# Set task dependencies
(
    start_task >> bigquery_all_blocknumbers >> generate_integers_array >> check_missing_values >> publish_range >> end_task
)
