# Features

## Synopsis

You can either define `--features` in the `Cargo.toml` file inside the `etl-core` repository or specify them as part of a command.

`cargo build --features ARGS...`
`cargo run --features ARGS...`

The `--features` option is required to build or run the ETL project.

## Arguments

Currently, the following blockchains are supported:
- `SONIC`

A message queue is required to be specified:
- `RABBITMQ` - a classic RabbitMQ queue
- `RABBITMQ_STREAM` - a RabbitMQ with Stream Queue plugin
- `GOOGLE_PUBSUB` - Google Cloud Pub/Sub

## Examples

1. Build the local project and its dependencies for the _SONIC_ blockchain
```
cargo build --release --features SONIC,RABBITMQ_STREAM
```

2. Run the local project and its dependencies for the _SONIC_blockchain and _RABBITMQ_STREAM_ exporter
```
cargo run --features SONIC,RABBITMQ_STREAM
```
