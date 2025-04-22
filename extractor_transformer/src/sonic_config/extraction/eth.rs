use std::marker::PhantomData;

use alloy::{
    network::Ethereum,
    providers::Provider,
    transports::{Transport, TransportResult},
};

use crate::blockchain_config::proto_codegen::etl::request::IndexingRequest;

use super::{
    traits::{EvmDebugExtractor, EvmExtractor, Extractor},
    EvmExtracted,
};

/// An [Extractor] implementation for EVM compatible chains.  
pub struct EthExtractor<T: Transport + Clone, P: Provider<T, Ethereum>> {
    provider: P,
    metrics: Option<crate::metrics::Metrics>,
    _transport: PhantomData<T>,
}

impl<T: Transport + Clone, P: Provider<T>> EthExtractor<T, P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            metrics: None,
            _transport: PhantomData,
        }
    }

    pub fn new_with_metrics(provider: P, metrics: Option<crate::metrics::Metrics>) -> Self {
        Self {
            provider,
            metrics,
            _transport: PhantomData,
        }
    }

    /// Extracts the basic data
    pub async fn extract_basic(
        &self,
        block_number: u64,
        request: Option<IndexingRequest>,
        n_retry: usize,
        cooldown: usize,
    ) -> TransportResult<Option<EvmExtracted>> {
        let request = request.unwrap_or_default();

        let (block_timestamp, block_hash, block) = {
            let block = self
                .get_block_with_retry(block_number, true, n_retry, cooldown)
                .await?;
            match block {
                Some(block) if (request.blocks || request.transactions) => (
                    block.header.timestamp,
                    block.header.hash.expect("Received block with no hash"),
                    Some(block),
                ),
                Some(block) => (
                    block.header.timestamp,
                    block.header.hash.expect("Received block with no hash"),
                    None,
                ),
                None => return Ok(None),
            }
        };

        let logs = {
            if request.logs || request.decoded_events || request.blocks {
                Some(
                    self.get_logs_with_retry(block_number, n_retry, cooldown)
                        .await?,
                )
            } else {
                None
            }
        };

        let receipts = {
            if request.receipts {
                Some(
                    self.get_block_receipts_with_retry(block_number, n_retry, cooldown)
                        .await?,
                )
            } else {
                None
            }
        };

        Ok(Some(EvmExtracted {
            block_number,
            block_hash,
            block_timestamp: block_timestamp as i64,
            receipts,
            logs,
            block,
        }))
    }
}

impl<T: Transport + Clone, P: Provider<T>> Extractor<P, T> for EthExtractor<T, P> {
    fn provider(&self) -> &P {
        &self.provider
    }
}

// Impl the EvmExtractor to allow calls for all basic data available.
impl<T: Transport + Clone, P: Provider<T>> EvmExtractor<P, T> for EthExtractor<T, P> {
    fn incr_fails(&self) {
        #[cfg(feature = "METRICS")]
        if let Some(metrics) = &self.metrics {
            metrics.failed_request_count.inc();
        }
    }

    fn incr_request(&self) {
        #[cfg(feature = "METRICS")]
        if let Some(metrics) = &self.metrics {
            metrics.request_count.inc();
        }
    }
}
// Impl the EvmDebugExtractor to allow trace calls (`trace_transaction`, `trace_block`)
impl<T: Transport + Clone, P: Provider<T>> EvmDebugExtractor<P, T> for EthExtractor<T, P> {}
