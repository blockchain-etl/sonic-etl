# ETL Infrastructure Architecture

## Architecture Framework
The `etl-core` repository will serve as the primary engine for ETL actions, operating at the network level and service level, and can accept custom configurations. Developers will be able to set up custom configurations within `etl-core`.  Further configurations are described in the [configurations](/docs/features.md) document. Once the network and export service is selected, users can use `etl-core` to export the desired blockchain data. The overall infrastructure is depicted below.

![architecture](/docs/img/architecture.png)


Currently, the Solana and Aptos blockchains are supported in [etl-solana-config](https://github.com/BCWResearch/etl-solana-config) and in [etl-aptos-config](https://github.com/BCWResearch/etl-aptos-config).

## Macro Infrastructure
An RPC node is expected to serve requests. Blocks are continually requested using the node, and if necessary, other data such as accounts may be requested as well. Upon response, the data is converted into a Protocol Buffers data format and sent to a streaming queue, such as Google Cloud Pub/Sub or RabbitMQ. You will need a transformer and loader that listens for the messages, transforms them to match the table schema, and inserts them into BigQuery.

The detailed extraction process is explained in the [extraction](/docs/extraction.md) document.
