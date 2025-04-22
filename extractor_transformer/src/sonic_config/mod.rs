// TODO: this file will contain the high-level logic (glue).
//  e.g. main() will call the function in this file for the indexing logic as well as the data extraction and record outputting

use std::{
    env::{self, VarError},
    fs::File,
    path::Path,
    time::Duration,
};

#[cfg(feature = "JSONL")]
pub mod test;

use crate::metrics::Metrics;

use super::output;

mod extraction;
pub mod proto_codegen;
mod proto_support;
pub mod streampublisher;
mod transformation;

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider},
    transports::{
        http::{Client, Http},
        RpcError, TransportErrorKind,
    },
};
use extraction::EvmDebugExtractor;
use log::{debug, error, info, warn};
use tokio::time::sleep;
use transformation::{
    bq::integer::TryIntoInteger,
    common::{
        set_event_count, transform_block, transform_logs_and_events, transform_receipts,
        transform_traces, transform_transactions,
    },
    err::TransformationErr,
    events::{ErcEventCatalog, EventCatalog},
};

use proto_codegen::etl::request::IndexingRequest;

pub const PROVIDER_URL_ENVKEY: &str = "PROVIDER_URL";
pub const FALLBACK_PROVIDER_URL_ENVKEY: &str = "FALLBACK_PROVIDER_URL";
pub const EXTRACT_N_RETRY_ENVKEY: &str = "EXTRACTION_N_RETRY";
pub const EXTRACT_RETRY_COOLDOWN_ENVKEY: &str = "EXTRACTION_RETRY_COOLDOWN";

/// This function creates a pubsub subscription to create requests for what tx versions we
/// want to index.
#[cfg(feature = "ORCHESTRATED")]
pub async fn subscribe_and_extract(
    pubsub_subscription: google_cloud_pubsub::subscription::Subscription,
    publisher: output::publish::StreamPublisher,
    metrics: Option<Metrics>,
) -> Result<(), Vec<(u64, ExtractTransformErr)>> {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use log::warn;
    use prost::Message;
    use tokio::signal::unix::{signal, SignalKind};

    info!("Starting the indexer...");

    // shared between the terminator thread for signal handling, and the main subscriber thread.
    let terminated = Arc::new(AtomicBool::new(false));

    let mut provider = Some(build_provider());

    let catalog = Some(ErcEventCatalog::default());

    // spawns a thread that just listens for a SIGTERM signal, and sets a shutdown flag upon receiving it.
    let terminator = terminated.clone();
    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to set up SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to set up SIGINT handler");

        // hangs until receiving the signal.
        //sigterm.recv().await;
        tokio::select! {
            _ = sigterm.recv() => {
                warn!("SIGTERM received, shutting down gracefully...");
            },
            _ = sigint.recv() => {
                warn!("SIGINT received, shutting down gracefully...");
            },
        }

        // set terminated flag to true to indicate a shutdown should occur.
        // uses release ordering because this is the writer thread.
        terminator.store(true, Ordering::Release);
    });

    // continually pulls pub/sub messages to determine which ranges to index.
    // uses acquire ordering for reader thread
    // TODO: add a timer to this .await and continue to the next iteration. this ensures that if there is no message in the pub/sub subscription, then this instance can still be shutdown (otherwise this .await will hang)

    while !terminated.load(Ordering::Acquire) {
        let message = match pubsub_subscription.pull(1, None).await {
            Ok(mut o) => match o.pop() {
                None => {
                    warn!("Didn't receive a message from the subscription. Retrying...");
                    continue;
                }
                Some(m) => m,
            },
            Err(e) => {
                warn!(
                    "Could not pull a pub/sub message from the subscription: {:?}. Retrying...",
                    e
                );
                continue;
            }
        };

        info!("Got Message: {:?}", message.message);

        // deserialize the pub/sub message into an indexing range
        let cur_request: IndexingRequest = {
            let message_ref: &[u8] = message.message.data.as_ref();
            IndexingRequest::decode(message_ref)
                .expect("pub/sub message uses the IndexingRange protobuf format")
        };

        match extract_transform_range(
            cur_request,
            publisher.clone(),
            metrics,
            provider.clone(),
            catalog.clone(),
        )
        .await
        {
            Err(extract_error) => {
                match build_active_provider().await {
                    Ok(new_provider) => provider = Some(new_provider),
                    Err(e) => error!("Failed to build new provider: {:?}", e),
                }

                match message.nack().await {
                    Ok(_) => error!("Nacked the message due to extraction or transformation error"),
                    Err(status) => {
                        error!("Nack returned a status: {:?}", status);
                    }
                };
                return Err(extract_error);
            }
            Ok(_) => info!("Extraction and transformation succeeded"),
        }

        // ack the message to prevent the message from being re-delivered.
        match message.ack().await {
            Ok(_) => info!("Acked the message"),
            Err(status) => {
                info!("Ack returned a status: {:?}", status);
                continue;
            }
        };
    }

    info!("Received a shutdown signal. Shutting down...");

    Ok(())
}

type EventCatalogType = ErcEventCatalog;

pub fn build_provider(
) -> RootProvider<alloy::transports::http::Http<alloy::transports::http::Client>> {
    sleep(Duration::from_secs(3));
    match env::var(PROVIDER_URL_ENVKEY) {
        Ok(raw_url) => {
            let rpc_url: alloy::transports::http::reqwest::Url = match raw_url.parse() {
                Ok(url) => url,
                Err(err) => panic!(
                    "Failed to parse url from env given key `{}`: {}",
                    PROVIDER_URL_ENVKEY, err
                ),
            };

            ProviderBuilder::new().on_http(rpc_url.clone())
        }
        Err(env::VarError::NotPresent) => {
            panic!("Missing `{}` envkey for provider", PROVIDER_URL_ENVKEY)
        }
        Err(env::VarError::NotUnicode(badstr)) => panic!(
            "Failed to decode env variable `{}`: {:?}",
            PROVIDER_URL_ENVKEY, badstr
        ),
    }
}

pub fn build_provider_fallback(
) -> RootProvider<alloy::transports::http::Http<alloy::transports::http::Client>> {
    sleep(Duration::from_secs(3));
    match env::var(FALLBACK_PROVIDER_URL_ENVKEY) {
        Ok(raw_url) => {
            let rpc_url: alloy::transports::http::reqwest::Url = match raw_url.parse() {
                Ok(url) => url,
                Err(err) => panic!(
                    "Failed to parse url from env given key `{}`: {}",
                    PROVIDER_URL_ENVKEY, err
                ),
            };

            ProviderBuilder::new().on_http(rpc_url.clone())
        }
        Err(env::VarError::NotPresent) => {
            panic!("Missing `{}` envkey for provider", PROVIDER_URL_ENVKEY)
        }
        Err(env::VarError::NotUnicode(badstr)) => panic!(
            "Failed to decode env variable `{}`: {:?}",
            PROVIDER_URL_ENVKEY, badstr
        ),
    }
}

pub async fn build_active_provider() -> Result<
    RootProvider<alloy::transports::http::Http<alloy::transports::http::Client>>,
    (RpcError<TransportErrorKind>, RpcError<TransportErrorKind>),
> {
    let primary = build_provider();

    if let Err(err1) = primary.get_client_version().await {
        warn!("Primary node failed: {}", err1);
        let secondary = build_provider_fallback();
        if let Err(err2) = secondary.get_client_version().await {
            warn!("Secondary node failed: {}", err2);
            Err((err1, err2))
        } else {
            Ok(secondary)
        }
    } else {
        Ok(primary)
    }
}

pub fn build_catalog() -> EventCatalogType {
    ErcEventCatalog::default()
}

pub async fn extract_transform_range(
    request: IndexingRequest,
    publisher: output::publish::StreamPublisher,
    metrics: Option<Metrics>,
    provider: Option<RootProvider<Http<Client>>>,
    catalog: Option<ErcEventCatalog>,
) -> Result<(), Vec<(u64, ExtractTransformErr)>> {
    info!(
        "Extracting & Transforming blocks [{},{})",
        request.start, request.end
    );

    let mut errors = Vec::new();

    let catalog = catalog.unwrap_or_default();

    for block_number in request.start..=request.end {
        let et_results = extract_transform(
            block_number,
            metrics.clone(),
            Some(request.clone()),
            provider.clone(),
            catalog.clone(),
        )
        .await;

        match et_results {
            Ok(perblock) => {
                debug!("Completed extract_transform block #{}", block_number);
                match publish_perblock_records(perblock, &publisher).await {
                    Ok(_) => info!(
                        "Extracted, Transformed, and Published for block #{}",
                        block_number
                    ),
                    Err(_) => error!(
                        "Failed to to publish after successful extract_transform for block #{}",
                        block_number
                    ),
                }
            }
            Err(err) => {
                error!(
                    "Failed to extract_transform block #{}: {:?}",
                    block_number, err
                );
                errors.push((block_number, err));
            }
        }
    }

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct PerBlockRecords {
    block_number: u64,
    block: Option<proto_codegen::etl::blocks::Block>,
    logs: Option<Vec<proto_codegen::etl::logs::Log>>,
    receipts: Option<Vec<proto_codegen::etl::receipts::Receipt>>,
    transactions: Option<Vec<proto_codegen::etl::transactions::Transaction>>,
    events: Option<Vec<proto_codegen::etl::decoded_events::DecodedEvent>>,
    traces: Option<Vec<proto_codegen::etl::traces::Trace>>,
}

/// The primary function for indexing, requests data and creates records to be sent to publishers based on the Sonic response.
pub async fn extract_transform<C: EventCatalog>(
    block_number: u64,
    metrics: Option<Metrics>,
    request: Option<IndexingRequest>,
    provider: Option<RootProvider<Http<Client>>>,
    catalog: C,
) -> Result<PerBlockRecords, ExtractTransformErr> {
    info!("Extracting & Transforming block #{}", block_number);

    let request = request.unwrap_or_default();

    let provider = match provider {
        Some(provider) => provider,
        None => build_provider(),
    };

    let extractor = extraction::EthExtractor::new_with_metrics(provider, metrics);

    let n_retry: usize = match std::env::var(EXTRACT_N_RETRY_ENVKEY) {
        Ok(n_rety) => match n_rety.parse() {
            Ok(n_retry) => n_retry,
            Err(err) => {
                error!(
                    "Failed to parse n_retry from envkey `{}`, (fallback to 1): {}  ",
                    EXTRACT_N_RETRY_ENVKEY, err
                );
                1
            }
        },
        Err(VarError::NotPresent) => 1,
        Err(VarError::NotUnicode(badstr)) => {
            error!(
                "Failed to parse n_retry from envkey `{}`, (fallback to 1): bad string '{:?}'  ",
                EXTRACT_N_RETRY_ENVKEY, badstr
            );
            1
        }
    };

    let cooldown: usize = match std::env::var(EXTRACT_RETRY_COOLDOWN_ENVKEY) {
        Ok(cooldown) => match cooldown.parse() {
            Ok(cooldown) => cooldown,
            Err(err) => {
                error!(
                    "Failed to parse retry_cooldown from envkey `{}`, (fallback to 5): {}  ",
                    EXTRACT_RETRY_COOLDOWN_ENVKEY, err
                );
                5
            }
        },
        Err(VarError::NotPresent) => 1,
        Err(VarError::NotUnicode(badstr)) => {
            error!(
                "Failed to parse cooldown from envkey `{}`, (fallback to 5): bad string '{:?}'  ",
                EXTRACT_RETRY_COOLDOWN_ENVKEY, badstr
            );
            5
        }
    };

    // Perform the extraction

    let mut records = PerBlockRecords {
        block_number,
        ..PerBlockRecords::default()
    };

    // =============================================================================================
    // Extraction
    // =============================================================================================
    let basic_extraction = match extractor
        .extract_basic(block_number, Some(request.clone()), n_retry, cooldown)
        .await?
    {
        Some(basic) => basic,
        None => return Err(ExtractTransformErr::ExtractorReturnedNone),
    };
    let debug_extraction = match extractor.extract_debug(block_number).await? {
        Some(debug) => debug,
        None => return Err(ExtractTransformErr::ExtractorReturnedNone),
    };

    // =============================================================================================
    // Blocks (w/o decoded events count) & Transactions
    // =============================================================================================

    if request.blocks {
        records.block = Some(transform_block(&basic_extraction)?);
    }

    if request.transactions {
        records.transactions = Some(transform_transactions(&basic_extraction)?);
    }

    // =============================================================================================
    // Traces
    // =============================================================================================

    if request.traces | request.blocks | request.transactions {
        let (traces, block_trace_cnt, per_tx_trace_cnt) = transform_traces(
            &debug_extraction,
            request.traces,
            request.blocks,
            request.transactions,
        )
        .await?;
        records.traces = traces;

        if request.blocks {
            if let Some(block) = &mut records.block {
                if let Some(trace_count) = block_trace_cnt {
                    block.trace_count = match trace_count.try_into_integer() {
                        Ok(casted) => casted,
                        Err(err) => {
                            return Err(TransformationErr::new(err.to_string(), None).into())
                        }
                    };
                }
            } else {
                return Err(ExtractTransformErr::Transformation(TransformationErr::new(
                    "Missing block trace count after transform_traces".to_string(),
                    None,
                )));
            }
        }

        if let Some(txs) = &mut records.transactions {
            if let Some(tx_trace_counts) = per_tx_trace_cnt {
                for tx in txs.iter_mut() {
                    if let Some(trace_count) =
                        tx_trace_counts.get(&Some(tx.transaction_index as u64))
                    {
                        tx.trace_count = *trace_count as i64;
                    }
                }
            }
        }
    }

    // =============================================================================================
    // Logs & Events (+ Block log/event counts)
    // =============================================================================================

    // If filling the decoded events, logs, or block tables, we need to review logs and events and
    // blocks
    if request.decoded_events | request.logs | request.blocks {
        let (maybe_log_records, maybe_event_records, decode_count) = transform_logs_and_events(
            &basic_extraction,
            request.logs,
            request.decoded_events,
            Some(catalog),
        )?;

        if request.logs {
            if maybe_log_records.is_none() {
                panic!("Panic")
            }
            records.logs = maybe_log_records;
        }

        if request.decoded_events {
            if maybe_event_records.is_none() {
                panic!("Panic");
            }
            records.events = maybe_event_records;
        }

        if let Some(block) = records.block {
            records.block = Some(set_event_count(block, decode_count));
        }
    }

    // =============================================================================================
    // Receipts
    // =============================================================================================

    if request.receipts {
        let receipts = transform_receipts(&basic_extraction).await?;
        records.receipts = Some(receipts);
    }

    Ok(records)
}

pub async fn publish_perblock_records(
    perblock: PerBlockRecords,
    publisher: &output::publish::StreamPublisher,
) -> Result<(), ()> {
    if let Some(block) = perblock.block {
        let timestamp = block.block_timestamp;
        publish_records(
            &publisher.blocks,
            vec![block],
            Some(&format!("{}", perblock.block_number)),
            vec![timestamp],
        )
        .await;
    }

    if let Some(events) = perblock.events {
        let timestamps = events.iter().map(|e| e.block_timestamp).collect::<Vec<_>>();
        publish_records(
            &publisher.decoded_events,
            events,
            Some(&format!("{}", perblock.block_number)),
            timestamps,
        )
        .await;
    }
    if let Some(logs) = perblock.logs {
        let timestamps = logs.iter().map(|l| l.block_timestamp).collect::<Vec<_>>();
        publish_records(
            &publisher.logs,
            logs,
            Some(&format!("{}", perblock.block_number)),
            timestamps,
        )
        .await;
    }
    if let Some(receipts) = perblock.receipts {
        let timestamps = receipts
            .iter()
            .map(|r| r.block_timestamp)
            .collect::<Vec<_>>();
        publish_records(
            &publisher.receipts,
            receipts,
            Some(&format!("{}", perblock.block_number)),
            timestamps,
        )
        .await;
    }
    if let Some(txs) = perblock.transactions {
        let timestamps = txs.iter().map(|tx| tx.block_timestamp).collect::<Vec<_>>();
        publish_records(
            &publisher.transactions,
            txs,
            Some(&format!("{}", perblock.block_number)),
            timestamps,
        )
        .await;
    }

    if let Some(traces) = perblock.traces {
        let timestamps = traces
            .iter()
            .map(|trace| trace.block_timestamp as i64)
            .collect::<Vec<_>>();
        publish_records(
            &publisher.traces,
            traces,
            Some(&format!("{}", perblock.block_number)),
            timestamps,
        )
        .await;
    }
    Ok(())
}

#[cfg(feature = "JSONL")]
pub async fn save_range(
    request: IndexingRequest,
    _metrics: Option<Metrics>,
    provider: Option<RootProvider<Http<Client>>>,
    dirpath: &Path,
) -> Result<(), Option<RpcError<TransportErrorKind>>> {
    for block_number in request.start..=request.end {
        save_block(block_number, provider.clone(), dirpath).await?;
    }
    Ok(())
}

pub async fn save_block(
    block_number: u64,
    provider: Option<RootProvider<Http<Client>>>,
    dirpath: &Path,
) -> Result<(), Option<RpcError<TransportErrorKind>>> {
    let extractor = extraction::EthExtractor::new(provider.unwrap_or(build_provider()));

    let basic = match extractor.extract_basic(block_number, None, 5, 5).await? {
        Some(basic) => basic,
        None => panic!("Failed to extract the block to save"),
    };

    let basic_file = File::create(dirpath.join(format!("basic_{}.json", block_number)))
        .expect("Failed to create file ");

    serde_json::to_writer(basic_file, &basic).expect("Failed to write basic extraction");

    let debug = match extractor.extract_debug(block_number).await? {
        Some(debug) => debug,
        None => panic!("Failed to extract the debug info for the block to save"),
    };

    let debug_file = File::create(dirpath.join(format!("debug_{}.json", block_number))).expect("");

    serde_json::to_writer(debug_file, &debug).expect("Failed to write debug extraction");

    Ok(())
}

#[derive(Debug)]
pub enum ExtractTransformErr {
    ExtractorReturnedNone,
    Rpc(RpcError<TransportErrorKind>),
    Transformation(TransformationErr),
}

impl From<TransformationErr> for ExtractTransformErr {
    fn from(value: TransformationErr) -> Self {
        Self::Transformation(value)
    }
}

impl From<RpcError<TransportErrorKind>> for ExtractTransformErr {
    #[inline]
    fn from(value: RpcError<TransportErrorKind>) -> Self {
        Self::Rpc(value)
    }
}

/// Publishes the records
#[allow(unused_variables)]
async fn publish_records<T>(
    publisher: &crate::output::publish::StreamPublisherConnection,
    records: Vec<T>,
    name: Option<&str>,
    timestamp: Vec<i64>,
) where
    T: prost::Message,
    T: serde::Serialize,
{
    debug!("Publishing: {:?}", name);
    #[cfg(feature = "PUBLISH_WITH_NAME")]
    if let Some(output_name) = name {
        #[cfg(feature = "JSON")]
        for (i, record) in records.into_iter().enumerate() {
            publisher
                .publish(&format!("{}_{}", output_name, i), record)
                .await;
        }
        #[cfg(feature = "JSONL")]
        publisher.publish_batch(output_name, records).await;
        #[cfg(feature = "GOOGLE_CLOUD_STORAGE")]
        publisher
            .publish_batch(output_name, timestamp, records)
            .await;
    }

    #[cfg(not(feature = "PUBLISH_WITH_NAME"))]
    {
        #[cfg(feature = "GOOGLE_PUBSUB")]
        publisher.publish_batch(records).await;

        #[cfg(not(feature = "GOOGLE_PUBSUB"))]
        for record in records {
            publisher.publish(record).await;
        }
    }
}
