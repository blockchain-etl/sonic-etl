use alloy::{
    eips::BlockNumberOrTag,
    network::{Ethereum, Network},
    primitives::FixedBytes,
    providers::Provider,
    rpc::types::{Block, Filter, Log, TransactionReceipt},
    transports::{BoxTransport, Transport, TransportResult},
};
use log::{info, warn};

use super::Extractor;

/// [EvmExtractor] is built to extract necessary information
/// for the ETL given the block_number, assuming we are operating
/// per block.
#[allow(async_fn_in_trait)]
pub trait EvmExtractor<
    P: Provider<T, N>,
    T: Transport + Clone = BoxTransport,
    N: Network = Ethereum,
>: Extractor<P, T, N>
{
    fn incr_request(&self);

    fn incr_fails(&self);

    /// Returns a [Block] given the `block_number` as a [u64]. `full` [bool] determines whether
    /// or not you will receive the transactions as well.
    async fn get_block(&self, block_number: u64, full: bool) -> TransportResult<Option<Block>> {
        self.provider()
            .get_block_by_number(BlockNumberOrTag::Number(block_number), full)
            .await
    }

    /// Like [EvmExtractor::get_block], except includes `n_retry` attempts, and a `cooldown`
    /// time period in seconds
    async fn get_block_with_retry(
        &self,
        block_number: u64,
        full: bool,
        n_retry: usize,
        cooldown: usize,
    ) -> TransportResult<Option<Block>> {
        if n_retry == 0 {
            self.get_block(block_number, full).await
        } else {
            let mut mr_attempt: TransportResult<Option<Block>> = Ok(None);
            for attempt_n in 0..n_retry {
                mr_attempt = self.get_block(block_number, full).await;
                match &mr_attempt {
                    Ok(Some(_)) => return mr_attempt,
                    Ok(None) => {
                        self.incr_fails();
                        warn!(
                            "Failed to retrieve block #{} (attempt #{}/{}), retrieved None.",
                            block_number, attempt_n, n_retry
                        );
                    }
                    Err(err) => {
                        self.incr_fails();
                        warn!(
                            "Failed to retrieve block #{} (attempt #{}/{}), error returned: {}",
                            block_number, attempt_n, n_retry, err
                        );
                    }
                }

                if cooldown > 0 {
                    info!(
                        "Due to failure to retrieve block #{}, will retry after {} seconds.",
                        block_number, cooldown
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(cooldown as u64)).await;
                }
            }

            mr_attempt
        }
    }

    /// Returns a [Vec] of [Log] given the `block_number` as a [u64]
    async fn get_logs(&self, block_number: u64) -> TransportResult<Vec<Log>> {
        self.incr_request();
        self.provider()
            .get_logs(&Filter::new().select(block_number))
            .await
    }

    /// Like [EvmExtractor::get_logs], except includes `n_retry` attempts, and a `cooldown`
    /// time period in seconds
    async fn get_logs_with_retry(
        &self,
        block_number: u64,
        n_retry: usize,
        cooldown: usize,
    ) -> TransportResult<Vec<Log>> {
        if n_retry == 0 {
            self.get_logs(block_number).await
        } else {
            let mut mr_attempt = Ok(Vec::new());

            for attempt_n in 0..n_retry {
                mr_attempt = self.get_logs(block_number).await;

                match &mr_attempt {
                    Ok(_) => return mr_attempt,
                    Err(err) => {
                        self.incr_fails();
                        warn!("Failed to retrieve logs from block #{} (attempt #{}/{}), error returned: {}", block_number, attempt_n, n_retry, err);
                    }
                }

                if cooldown > 0 {
                    info!("Due to failure to retrieve logs from block #{}, will retry after {} seconds.", block_number, cooldown);
                    tokio::time::sleep(std::time::Duration::from_secs(cooldown as u64)).await;
                }
            }

            mr_attempt
        }
    }

    /// Returns a [Vec] of ReceiptResponses (Network-dependent) given the `block_number` as a [u64]
    async fn get_block_receipts(
        &self,
        block_number: u64,
    ) -> TransportResult<Vec<N::ReceiptResponse>> {
        self.incr_request();
        self.provider()
            .get_block_receipts(block_number.into())
            .await
            .map(|opt| opt.unwrap_or_default())
    }

    /// Like [EvmExtractor::get_block_receipts], except includes `n_retry` attempts, and a `cooldown`
    /// time period in seconds
    async fn get_block_receipts_with_retry(
        &self,
        block_number: u64,
        n_retry: usize,
        cooldown: usize,
    ) -> TransportResult<Vec<N::ReceiptResponse>> {
        if n_retry == 0 {
            self.get_block_receipts(block_number).await
        } else {
            let mut mr_attempt = Ok(Vec::new());

            for attempt_n in 0..n_retry {
                mr_attempt = self.get_block_receipts(block_number).await;

                match &mr_attempt {
                    Ok(_) => return mr_attempt,
                    Err(err) => {
                        self.incr_fails();
                        warn!("Failed to retrieve receipts from block #{} (attempt ${}/{}), error returned: {}", block_number, attempt_n, n_retry, err);
                    }
                }

                if cooldown > 0 {
                    info!("Due to failure to retrieve receipts from block #{}, will retry after {} seconds.", block_number, cooldown);
                    tokio::time::sleep(std::time::Duration::from_secs(cooldown as u64)).await;
                }
            }

            mr_attempt
        }
    }
}

pub trait EvmExtraction: serde::Serialize + Clone {
    fn block_number(&self) -> u64;

    fn block_hash(&self) -> FixedBytes<32>;

    fn block_timestamp(&self) -> i64;

    fn block(&self) -> Option<&Block>;

    fn logs(&self) -> Option<&Vec<Log>>;

    fn receipts(&self) -> Option<&Vec<TransactionReceipt>>;
}

/// A struct containing all pieces of an EVM block that can be indexed (minus debug)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EvmExtracted {
    pub block_number: u64,
    pub block_hash: FixedBytes<32>,
    pub block_timestamp: i64,
    pub block: Option<Block>,
    pub logs: Option<Vec<Log>>,
    pub receipts: Option<Vec<TransactionReceipt>>,
}

impl EvmExtraction for EvmExtracted {
    /// Returns a reference to the block data
    fn block(&self) -> Option<&Block> {
        self.block.as_ref()
    }
    /// Returns the block_hash
    fn block_hash(&self) -> FixedBytes<32> {
        self.block_hash
    }
    /// Returns the block_number
    fn block_number(&self) -> u64 {
        self.block_number
    }
    /// Returns the block_timestamp as a [i64]
    fn block_timestamp(&self) -> i64 {
        self.block_timestamp
    }
    /// Returns the vector of logs if applicable
    fn logs(&self) -> Option<&Vec<Log>> {
        self.logs.as_ref()
    }
    /// Returns receipts if applicable
    fn receipts(&self) -> Option<&Vec<TransactionReceipt>> {
        self.receipts.as_ref()
    }
}
