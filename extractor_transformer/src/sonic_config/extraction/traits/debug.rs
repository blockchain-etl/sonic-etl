use alloy::{
    network::{Ethereum, Network},
    primitives::{Address, BlockHash, BlockNumber, TxHash},
    providers::Provider,
    transports::{BoxTransport, Transport, TransportResult},
};
use serde::{Deserialize, Serialize};

use super::Extractor;

/// Allows extractor trace debug data from chains
#[allow(async_fn_in_trait)]
pub trait EvmDebugExtractor<
    P: Provider<T, N>,
    T: Transport + Clone = BoxTransport,
    N: Network = Ethereum,
>: Extractor<P, T, N>
{
    #[inline]
    async fn extract_debug(&self, block_number: u64) -> TransportResult<Option<DebugTraces>> {
        let provider = self.provider();

        let (block_hash, block_timestamp) = match provider
            .get_block(
                block_number.into(),
                alloy::rpc::types::BlockTransactionsKind::Hashes,
            )
            .await? {
                Some(block) => (block.header.hash, block.header.timestamp),
                None => return Ok(None),
            };

        let traces = if block_number != 0 {
            self.get_block_traces(block_number).await?
        } else {
            Vec::new()
        };

        Ok(Some(DebugTraces {
            block_number,
            block_hash,
            block_timestamp: block_timestamp as i64,
            traces
        }))
    }

    /// Returns the transaction trace for a given transaction hash
    #[allow(dead_code)]
    #[inline]
    async fn get_tx_trace(&self, tx_hash: TxHash) -> TransportResult<Vec<TxTrace>> {
        self.provider()
            .raw_request::<_, Vec<TxTrace>>("trace_transaction".into(), vec![tx_hash])
            .await
    }

    /// Returns all the transaction traces within a singular block
    #[inline]
    async fn get_block_traces(&self, block_number: u64) -> TransportResult<Vec<TxTrace>> {
        self.provider()
            .raw_request::<_, Vec<TxTrace>>(
                "trace_block".into(),
                vec![format!("0x{:x}", block_number)],
            )
            .await
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CallType {
    Call,
    DelegateCall,
    StaticCall,
    CallCode,
}

impl std::fmt::Display for CallType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Call => write!(f, "call"),
            Self::DelegateCall => write!(f, "delegatecall"),
            Self::StaticCall => write!(f, "staticcall"),
            Self::CallCode => write!(f, "callcode"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TraceType {
    Call,
    Create,
    Reward,
    Suicide,
    Empty,
}

impl std::fmt::Display for TraceType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Call => write!(f, "call"),
            Self::Create => write!(f, "create"),
            Self::Reward => write!(f, "reward"),
            Self::Suicide => write!(f, "suicide"),
            Self::Empty => write!(f, "empty"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum RewardType {
    Block,
    Uncle,
}

impl std::fmt::Display for RewardType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block => write!(f, "block"),
            Self::Uncle => write!(f, "uncle"),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct TxTraceActionCall {
    pub from: Address,
    #[serde(rename = "callType")]
    pub call_type: String,
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity")]
    pub gas: u128,
    pub input: String,
    pub to: Address,
    pub value: String,
    // #[serde(rename = "rewardType")]
    // pub reward_type: RewardType,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct TxTraceActionReward {
    pub author: Address,
    #[serde(rename = "rewardType")]
    pub reward_type: String,
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity")]
    pub value: u128,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TxTraceActionCreate {
    pub from: Address,
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity")]
    pub value: u128,
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity")]
    pub gas: u128,
    pub init: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TxTraceActionSuicide {
    #[serde(rename = "refundAddress")]
    pub refund_address: Option<Address>,
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity")]
    pub balance: u128,
    #[serde(rename = "address")]
    pub self_destructed_address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceResult {
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity")]
    pub gas_used: u128,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceResultCreate {
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity")]
    pub gas_used: u128,
    pub address: Address,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceResultEmpty {
    #[serde(default, with = "alloy::rpc::types::serde_helpers::quantity", rename = "gasUsed")]
    pub gas_used: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceCall {
    pub action: TxTraceActionCall,
    #[serde(rename = "blockHash")]
    pub block_hash: BlockHash,
    #[serde(rename = "blockNumber")]
    pub block_number: BlockNumber,
    pub error: Option<String>,
    pub result: Option<TxTraceResult>,
    pub subtraces: u64,
    #[serde(rename = "traceAddress")]
    pub trace_address: Vec<u64>,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: TxHash,
    #[serde(rename = "transactionPosition")]
    pub transaction_position: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceReward {
    pub action: TxTraceActionReward,
    #[serde(rename = "blockHash")]
    pub block_hash: BlockHash,
    #[serde(rename = "blockNumber")]
    pub block_number: BlockNumber,
    pub error: Option<String>,
    pub result: Option<TxTraceResult>,
    pub subtraces: u64,
    #[serde(rename = "traceAddress")]
    pub trace_address: Vec<u64>,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<TxHash>,
    #[serde(rename = "transactionPosition")]
    pub transaction_position: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceSuicide {
    pub action: TxTraceActionSuicide,
    #[serde(rename = "blockHash")]
    pub block_hash: BlockHash,
    #[serde(rename = "blockNumber")]
    pub block_number: BlockNumber,
    pub error: Option<String>,
    pub subtraces: u64,
    #[serde(rename = "traceAddress")]
    pub trace_address: Vec<u64>,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: TxHash,
    #[serde(rename = "transactionPosition")]
    pub transaction_position: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceCreate {
    #[serde(rename = "blockHash")]
    pub block_hash: BlockHash,
    #[serde(rename = "blockNumber")]
    pub block_number: BlockNumber,
    pub action: TxTraceActionCreate,
    pub result: Option<TxTraceResultCreate>,
    pub error: Option<String>,
    pub subtraces: u64,
    #[serde(rename = "transactionPosition")]
    pub transaction_position: u64,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: TxHash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TxTraceEmpty {
    #[serde(rename = "blockHash")]
    pub block_hash: BlockHash,
    #[serde(rename = "blockNumber")]
    pub block_number: BlockNumber,
    pub action: TxTraceActionCreate,
    pub result: Option<TxTraceResultEmpty>,
    pub error: Option<String>,
    pub subtraces: u64,
    #[serde(rename = "transactionPosition")]
    pub transaction_position: u64,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: TxHash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TxTrace {
    #[serde(rename = "call")]
    Call(TxTraceCall),
    #[serde(rename = "reward")]
    Reward(TxTraceReward),
    #[serde(rename = "create")]
    Create(TxTraceCreate),
    #[serde(rename = "suicide")]
    Suicide(TxTraceSuicide),
    #[serde(rename = "empty")]
    Empty(TxTraceEmpty),
}

#[allow(dead_code)]
impl TxTrace {
    #[inline]
    pub fn is_call(&self) -> bool {
        matches!(self, Self::Call(_))
    }

    #[inline]
    pub fn is_reward(&self) -> bool {
        matches!(self, Self::Reward(_))
    }

    #[inline]
    pub fn is_create(&self) -> bool {
        matches!(self, Self::Create(_))
    }

    #[inline]
    pub fn is_suicide(&self) -> bool {
        matches!(self, Self::Suicide(_))
    }

    pub fn unwrap_call(self) -> TxTraceCall {
        match self {
            Self::Call(call) => call,
            other => panic!(
                "Cannot `unwrap_call` given a TxTrace of other variant: {:?}",
                other
            ),
        }
    }

    pub fn unwrap_reward(self) -> TxTraceReward {
        match self {
            Self::Reward(reward) => reward,
            other => panic!(
                "Cannot `unwrap_reward` given a TxTrace of other variant: {:?}",
                other
            ),
        }
    }

    #[inline]
    pub const fn get_tx_info(&self) -> (Option<TxHash>, Option<u64>) {  
        match self {
            Self::Call(call) => (Some(call.transaction_hash), Some(call.transaction_position)),
            Self::Create(create) => (Some(create.transaction_hash), Some(create.transaction_position)),
            Self::Reward(reward) => (reward.transaction_hash, Some(reward.transaction_position)),
            Self::Suicide(suicide) => (Some(suicide.transaction_hash), Some(suicide.transaction_position)),
            Self::Empty(empty) => (Some(empty.transaction_hash), Some(empty.transaction_position))
        }
    }
}

pub trait DebugExtraction: serde::Serialize + Clone {
    fn block_number(&self) -> u64;

    fn block_hash(&self) -> Option<BlockHash>;

    fn block_timestamp(&self) -> i64;

    fn traces(&self) -> Vec<TxTrace>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DebugTraces {
    block_number: u64,
    block_hash: Option<BlockHash>,
    block_timestamp: i64,
    traces: Vec<TxTrace>,
}

impl DebugExtraction for DebugTraces {
    fn block_number(&self) -> u64 {
        self.block_number
    }

    fn block_hash(&self) -> Option<BlockHash> {
        self.block_hash
    }
    
    fn block_timestamp(&self) -> i64 {
        self.block_timestamp
    }

    fn traces(&self) -> Vec<TxTrace> {
        self.traces.clone()
    }
}

#[allow(dead_code)]
impl DebugTraces {
    #[inline]
    pub fn unwrap(self) -> Vec<TxTrace> {
        self.traces
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &TxTrace> {
        self.traces.iter()
    }
}
