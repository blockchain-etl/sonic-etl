# Environmental Variables

## Synopsis

You can define enviornmental variables in a `.env` file. Examples are illustrated in `.env.example.`

## Variables
- `ENDPOINT`  
**Required**. Specifies the address to use for json RPC requests.

- `FALLBACK_ENDPOINT`
**Required**. Specifies the address to use for json RPC requests, when the primary endpoint is failing. This value can be the same `ENDPOINT`.

- `NUM_EXTRACTOR_THREADS`  
**Required**. Specifies the number of concurrent threads to run an extract job.

- `ENABLE_METRICS`  
**Required**. This variable determines whether to launch a metrics server to collect metrics for Prometheus.

- `METRICS_ADDRESS`  
Optional. Required only if `ENABLE_METRICS` is true. Specifies the address of the metrics server.

- `METRICS_PORT`  
Optional. Required only if `ENABLE_METRICS` is true. Specifies the port of the metrics server.

- `RABBITMQ_ADDRESS`  
Optional. Required only if _STREAM_EXPORTER_  is set to `RABBITMQ_STREAM`. Specifies the address of RabbitMQ.

- `RABBITMQ_PORT`  
Optional. Required only if _STREAM_EXPORTER_  is set to `RABBITMQ_STREAM`. Specifies the port of RabbitMQ.

- `BIGTABLE_CRED`  
Optional. Specifies the file path of the credential file required to access GCP Bigtable.

- `GCP_CREDENTIALS_JSON_PATH`  
Optional. Required only if _STREAM_EXPORTER_  is set to `GOOGLE_PUBSUB`. Specifies the file path of the credential file required to access Google Pubsub.

- `GOOGLE_PUBSUB_TOPIC`  
Optional. Required only if _STREAM_EXPORTER_ is set to `GOOGLE_PUBSUB`. Specifies the Google Pubsub topic to be used during exporting. It is assumed that the PubSub Topic is already created.