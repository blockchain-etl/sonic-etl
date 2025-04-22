use std::collections::HashMap;
use std::str::FromStr;

use alloy::primitives::{FixedBytes, Uint, U256};
use log::{debug, info};

use crate::blockchain_config::extraction::{DebugExtraction, TxTrace};
use crate::blockchain_config::transformation::err::TransformationErr;
use crate::blockchain_config::transformation::{bq::integer::TryIntoInteger, events::EventCatalog};
use crate::blockchain_config::ExtractTransformErr;

use super::proto_codegen::etl::{
    blocks::{block::Withdrawal, Block},
    decoded_events::DecodedEvent,
    logs::Log,
    receipts::Receipt,
    traces::{
        trace::{TraceAction, TraceResult},
        Trace,
    },
    transactions::{transaction::AddressStorageKeyPair, Transaction},
};
use super::EvmExtraction;

#[inline]
pub fn format_fixed_bytes<const N: usize>(fixedbytes: FixedBytes<N>) -> String {
    format!("{}", fixedbytes)
}

pub fn transform_block<T: EvmExtraction>(extracted: &T) -> Result<Block, ExtractTransformErr> {
    debug!("Handling block #{}", extracted.block_number());
    let extracted_block = match extracted.block() {
        Some(block) => block,
        None => {
            return Err(TransformationErr::new(
                "Missing critical extractions for this transformation".to_string(),
                Some("block".to_string()),
            )
            .into())
        }
    };
    Ok(Block {
        block_hash: format_fixed_bytes(extracted.block_hash()),
        block_number: extracted.block_number() as i64,
        block_timestamp: extracted.block_timestamp(),
        parent_hash: format_fixed_bytes(extracted_block.header.parent_hash),
        sha3_uncles: format_fixed_bytes(extracted_block.header.uncles_hash),
        miner: format_fixed_bytes(extracted_block.header.miner.0),
        state_root: format_fixed_bytes(extracted_block.header.state_root),
        transaction_root: format_fixed_bytes(extracted_block.header.transactions_root),
        receipts_root: format_fixed_bytes(extracted_block.header.receipts_root),
        logs_bloom: format_fixed_bytes(extracted_block.header.logs_bloom.0),
        difficulty: format!("{}", extracted_block.header.difficulty),
        gas_limit: format!("{}", extracted_block.header.gas_limit),
        gas_used: format!("{}", extracted_block.header.gas_used),
        uncles: extracted_block
            .uncles
            .iter()
            .map(|item| format_fixed_bytes(*item))
            .collect::<Vec<String>>(),
        total_difficulty: extracted_block
            .header
            .total_difficulty
            .map(|diff| format!("{}", diff)),
        mix_hash: extracted_block.header.mix_hash.map(format_fixed_bytes),
        nonce: extracted_block.header.nonce.map(format_fixed_bytes),
        base_fee_per_gas: extracted_block
            .header
            .base_fee_per_gas
            .map(|item| format!("{}", item)),
        withdrawals_root: extracted_block
            .header
            .withdrawals_root
            .map(format_fixed_bytes),
        parent_beacon_block_root: extracted_block
            .header
            .parent_beacon_block_root
            .map(format_fixed_bytes),
        transactions_count: match extracted_block.transactions.len().try_into_integer() {
            Ok(item) => item,
            Err(err) => {
                return Err(TransformationErr::new(
                    err.to_string(),
                    Some("transactions_count".to_string()),
                )
                .into())
            }
        },
        size: match extracted_block.size.map(|size| size.try_into_integer()) {
            Some(Ok(size_integer)) => size_integer,
            Some(Err(err)) => {
                return Err(
                    TransformationErr::new(err.to_string(), Some("size".to_string())).into(),
                )
            }
            None => {
                return Err(TransformationErr::new(
                    "Missing size".to_string(),
                    Some("size".to_string()),
                )
                .into())
            }
        },
        withdrawals: {
            match &extracted_block.withdrawals {
                Some(withdrawals) => withdrawals
                    .iter()
                    .map(|w| Withdrawal::try_from(*w))
                    .collect::<Result<Vec<_>, TransformationErr>>()?,
                None => Vec::new(),
            }
        },
        extra_data: format!("{}", extracted_block.header.extra_data),
        log_count: match extracted.logs() {
            Some(logs) => logs.len() as i64,
            None => {
                return Err(TransformationErr::new(
                    "Missing logs from extracted".to_string(),
                    Some("Logs".to_string()),
                )
                .into())
            }
        },
        trace_count: 0,         // Value gets filled in later
        decoded_event_count: 0, // Value gets filled in later
        epoch: extract_tarnsform_epoch(extracted_block)
            .expect("Failed to parse epoch")
            .expect("Missing epoch in block response"),
    })
}

pub fn extract_tarnsform_epoch(
    extracted_block: &alloy::rpc::types::Block,
) -> Result<Option<i64>, std::num::ParseIntError> {
    match extracted_block.other.get("epoch") {
        Some(serde_json::Value::String(string)) => {
            let trimmed = string.strip_prefix("0x").unwrap_or(string);
            match i64::from_str_radix(trimmed, 16) {
                Ok(epoch_num) => Ok(Some(epoch_num)),
                Err(err) => Err(err),
            }
        }
        None => Ok(None),
        _ => unreachable!("Unexpected response, expected a hex"),
    }
}

/// Determines whether a number is within BigNumeric range
pub fn within_bignumeric_range(uint: &Uint<256, 4>) -> bool {
    uint.bit_len() < 97
        || uint <= &U256::from_str("99999999999999999999999999999999999999").unwrap()
}

pub fn cap_bignumeric(uint: &Uint<256, 4>) -> String {
    if within_bignumeric_range(uint) {
        uint.to_string()
    } else {
        "99999999999999999999999999999999999999".to_string()
    }
}

pub fn transform_transactions<T: EvmExtraction>(
    extracted: &T,
) -> Result<Vec<Transaction>, ExtractTransformErr> {
    debug!("Handling transactions #{}", extracted.block_number());
    let extracted_block = match extracted.block() {
        Some(block) => block,
        None => {
            return Err(TransformationErr::new(
                "Missing critical extractions for this transformation".to_string(),
                Some("block".to_string()),
            )
            .into())
        }
    };
    let capacity = extracted_block.transactions.len();
    let mut transactions = Vec::with_capacity(capacity);

    let txs = match extracted_block.transactions.as_transactions() {
        Some(txs) => txs,
        None => {
            return Err(TransformationErr::new(
                "Expected extracted.block.transactions.is_full() to be returned as true"
                    .to_string(),
                Some("block.transactions".to_string()),
            )
            .into())
        }
    };

    for (index, tx) in txs.iter().enumerate() {
        transactions.push(Transaction {
            block_hash: format_fixed_bytes(extracted.block_hash()),
            block_number: extracted.block_number() as i64,
            block_timestamp: extracted.block_timestamp(),
            transaction_hash: format!("{}", tx.hash),
            transaction_index: match tx.transaction_index {
                Some(index) => index as i64,
                None => index as i64,
            },
            nonce: match tx.nonce.try_into_integer() {
                Ok(nonce) => nonce,
                Err(err) => {
                    return Err(
                        TransformationErr::new(err.to_string(), Some("nonce".to_string())).into(),
                    )
                }
            },
            from_address: format!("{}", tx.from),
            to_address: tx.to.map(|to| format!("{}", to)),
            value: cap_bignumeric(&tx.value),
            value_lossless: format!("{}", tx.value),
            gas_price: tx.gas_price.map(|price| price as i64),
            gas: tx.gas as i64,
            max_fee_per_gas: match tx.max_fee_per_gas {
                Some(mfpg_u128) => match mfpg_u128.try_into_integer_opt() {
                    Ok(mfpg_i64) => mfpg_i64,
                    Err(err) => {
                        return Err(TransformationErr::new(
                            err.to_string(),
                            Some("max_fee_per_gas".to_string()),
                        )
                        .into())
                    }
                },
                None => None,
            },
            max_priority_fee_per_gas: match tx.max_priority_fee_per_gas {
                Some(mpfpg_u128) => match mpfpg_u128.try_into_integer_opt() {
                    Ok(mpfpg_i64) => mpfpg_i64,
                    Err(err) => {
                        return Err(TransformationErr::new(
                            err.to_string(),
                            Some("max_priority_fee_per_gas".to_string()),
                        )
                        .into())
                    }
                },
                None => None,
            },
            input: format!("{}", tx.input),
            transaction_type: match tx.transaction_type {
                Some(txtype) => txtype as u32,
                None => {
                    return Err(TransformationErr::new(
                        "Missing tx.transaction_type".to_string(),
                        Some("transaction_type".to_string()),
                    )
                    .into())
                }
            },
            chain_id: match tx.chain_id.map(|id| id.try_into_integer_opt()) {
                Some(Ok(chain_id)) => chain_id,
                Some(Err(err)) => {
                    return Err(TransformationErr::new(
                        err.to_string(),
                        Some("chain_id".to_string()),
                    )
                    .into())
                }
                None => None,
            },
            access_list: match &tx.access_list {
                Some(accesslist) => accesslist
                    .iter()
                    .map(|access_item| AddressStorageKeyPair {
                        address: Some(format!("{}", access_item.address)),
                        storage_keys: access_item
                            .storage_keys
                            .iter()
                            .map(|key| format!("{}", key))
                            .collect::<Vec<String>>(),
                    })
                    .collect::<Vec<_>>(),
                None => Vec::new(),
            },
            r: tx
                .signature
                .map(|sig| format!("0x{}", hex::encode(sig.r.to_be_bytes_vec()))),
            s: tx
                .signature
                .map(|sig| format!("0x{}", hex::encode(sig.s.to_be_bytes_vec()))),
            v: tx
                .signature
                .map(|sig| format!("0x{}", hex::encode(sig.v.to_be_bytes_vec()))),
            y_parity: match tx.signature {
                Some(sig) => {
                    if sig
                        .y_parity
                        .map(|y| y.0)
                        .unwrap_or((sig.v % Uint::from(2)) == Uint::from(1))
                    {
                        Some("0x1".to_string())
                    } else {
                        Some("0x0".to_string())
                    }
                }
                None => None,
            },
            trace_count: 0,
        });
    }
    Ok(transactions)
}

use crate::blockchain_config::transformation::events::{GetEventBySigErr, LogDecodeErr};

#[allow(clippy::type_complexity)]
pub fn transform_logs_and_events<T: EvmExtraction, C: EventCatalog>(
    extracted: &T,
    incl_logs: bool,
    incl_events: bool,
    catalog: Option<C>,
) -> Result<(Option<Vec<Log>>, Option<Vec<DecodedEvent>>, usize), ExtractTransformErr> {
    debug!("Handling Logs & Events #{}", extracted.block_number());

    let extracted_logs = match extracted.logs() {
        Some(logs) => logs,
        None => {
            return Err(TransformationErr::new(
                "Expected extracted.block.transactions.is_full() to be returned as true"
                    .to_string(),
                Some("block.transactions".to_string()),
            )
            .into())
        }
    };

    let mut logs = match incl_logs {
        true => Some(Vec::with_capacity(extracted_logs.len())),
        false => {
            info!("Skipping logs");
            None
        }
    };

    // Create a vector to store the decoded events
    let mut events: Option<Vec<DecodedEvent>> = match incl_events {
        true => Some(Vec::with_capacity(extracted_logs.len())),
        false => {
            info!("Skipping events");
            None
        }
    };

    let mut event_count: usize = 0;

    if incl_events && catalog.is_none() {
        panic!("Need to provide catalog if handling decoded events");
    }

    for log in extracted_logs.iter() {
        if let Some(logs) = &mut logs {
            logs.push(Log {
                block_hash: match log.block_hash {
                    Some(block_hash) => format!("{}", block_hash),
                    None => format!("{}", extracted.block_hash()),
                },
                block_number: extracted.block_number() as i64,
                block_timestamp: extracted.block_timestamp(),
                transaction_hash: match log.transaction_hash {
                    Some(hash) => format!("{}", hash),
                    None => format!("{}", extracted.block_hash()),
                },
                transaction_index: match log.transaction_index.map(|index| index.try_into_integer())
                {
                    Some(Ok(index)) => index,
                    Some(Err(err)) => {
                        return Err(TransformationErr::new(
                            err.to_string(),
                            Some("transaction_index".to_string()),
                        )
                        .into())
                    }
                    None => {
                        return Err(TransformationErr::new(
                            "Missing transaction_index".to_string(),
                            Some("transaction_index".to_string()),
                        )
                        .into())
                    }
                },
                log_index: match log.log_index.map(|index| index.try_into_integer()) {
                    Some(Ok(index)) => index,
                    Some(Err(err)) => {
                        return Err(TransformationErr::new(
                            err.to_string(),
                            Some("log_index".to_string()),
                        )
                        .into())
                    }
                    None => {
                        return Err(TransformationErr::new(
                            "Missing log_index".to_string(),
                            Some("log_index".to_string()),
                        )
                        .into())
                    }
                },
                address: Some(format!("{}", log.address().0)),
                data: Some(format!("0x{:?}", log.data().data)),
                topics: log
                    .data()
                    .topics()
                    .iter()
                    .map(|topic| format!("{}", topic))
                    .collect::<Vec<String>>(),
                removed: Some(log.removed),
            });
        }

        if let Some(catalog) = &catalog {
            match catalog.attempt_decode_log(log) {
                Ok(event) => {
                    event_count += 1;
                    if let Some(events) = &mut events {
                        events.push(DecodedEvent {
                            block_hash: match log.block_hash {
                                Some(block_hash) => format!("{}", block_hash),
                                None => format!("{}", extracted.block_hash()),
                            },
                            block_number: extracted.block_number() as i64,
                            block_timestamp: match log.block_timestamp {
                                Some(block_timestamp) => block_timestamp as i64,
                                None => extracted.block_timestamp(),
                            },
                            transaction_hash: match log.transaction_hash {
                                Some(tx_hash) => format!("{}", tx_hash),
                                None => {
                                    return Err(TransformationErr::new(
                                        "Missing critical extractions for this transformation"
                                            .to_string(),
                                        Some("transaction_hash".to_string()),
                                    )
                                    .into())
                                }
                            },
                            transaction_index: match log.transaction_index {
                                Some(tx_index) => tx_index as i64,
                                None => {
                                    return Err(TransformationErr::new(
                                        "Missing critical extractions for this transformation"
                                            .to_string(),
                                        Some("transaction_index".to_string()),
                                    )
                                    .into())
                                }
                            },
                            log_index: match log.log_index {
                                Some(log_index) => log_index as i64,
                                None => {
                                    return Err(TransformationErr::new(
                                        "Missing critical extractions for this transformation"
                                            .to_string(),
                                        Some("log_index".to_string()),
                                    )
                                    .into())
                                }
                            },
                            address: Some(format!("{}", log.address())),
                            event_hash: Some(format!("{}", event.event.selector())),
                            event_signature: Some(event.event.signature()),
                            topics: log
                                .topics()
                                .iter()
                                .map(|b256| format!("{}", b256))
                                .collect::<Vec<_>>(),
                            args: match event.args_to_json() {
                                Ok(json) => match serde_json::to_string(&json) {
                                    Ok(string) => Some(string),
                                    Err(err) => {
                                        unreachable!("Failed to turn json into string: {}", err)
                                    }
                                },
                                Err(err) => unreachable!(
                                    "Failed to convert decoded arguments into json: {:?}",
                                    err
                                ),
                            },
                            removed: Some(log.removed),
                        });
                    }
                }
                Err(LogDecodeErr::EventRetrievalErr(GetEventBySigErr::NotFound)) => {}
                Err(LogDecodeErr::LogHasNoTopics) => {}
                Err(err) => {
                    return Err(ExtractTransformErr::Transformation(TransformationErr::new(
                        err.to_string(),
                        None,
                    )))
                }
            }
        }
    }
    Ok((logs, events, event_count))
}

pub fn set_event_count(block: Block, event_count: usize) -> Block {
    Block {
        decoded_event_count: event_count as i64,
        ..block
    }
}

pub async fn transform_receipts<T: EvmExtraction>(
    extracted: &T,
) -> Result<Vec<Receipt>, ExtractTransformErr> {
    let extracted_receipts = match extracted.receipts() {
        Some(receipts) => receipts,
        None => {
            return Err(TransformationErr::new(
                "Missing receipts from extraction".to_string(),
                None,
            )
            .into())
        }
    };

    let mut vec = Vec::with_capacity(extracted_receipts.len());

    for receipt in extracted_receipts.iter() {
        vec.push(Receipt {
            block_hash: match receipt.block_hash {
                Some(hash) => format!("{}", hash),
                None => {
                    return Err(TransformationErr::new(
                        "Missing critical extractions for this transformation".to_string(),
                        Some("block_hash".to_string()),
                    )
                    .into())
                }
            },
            block_number: extracted.block_number() as i64,
            block_timestamp: extracted.block_timestamp(),
            transaction_hash: format!("{}", receipt.transaction_hash),
            transaction_index: match receipt.transaction_index.map(|i| i.try_into_integer()) {
                Some(Ok(fixed_idx)) => fixed_idx,
                Some(Err(err)) => {
                    return Err(TransformationErr::new(
                        err.to_string(),
                        Some("transaction_index".to_string()),
                    )
                    .into())
                }
                None => {
                    return Err(TransformationErr::new(
                        "No transaction index in receipt".to_string(),
                        Some("transaction_index".to_string()),
                    )
                    .into())
                }
            },
            from_address: format!("{}", receipt.from),
            to_address: receipt.to.map(|to| format!("{}", to)),
            contract_address: receipt.contract_address.map(|addr| format!("{}", addr)),
            cumulative_gas_used: match receipt.inner.cumulative_gas_used().try_into_integer() {
                Ok(gas) => gas,
                Err(err) => {
                    return Err(TransformationErr::new(
                        err.to_string(),
                        Some("cumulative_gas_used".to_string()),
                    )
                    .into())
                }
            },
            gas_used: match receipt.gas_used.try_into_integer() {
                Ok(gas) => gas,
                Err(err) => {
                    return Err(TransformationErr::new(
                        err.to_string(),
                        Some("gas_used".to_string()),
                    )
                    .into())
                }
            },
            effective_gas_price: match receipt.effective_gas_price.try_into_integer() {
                Ok(price) => price,
                Err(err) => {
                    return Err(TransformationErr::new(
                        err.to_string(),
                        Some("effective_gas_price".to_string()),
                    )
                    .into())
                }
            },
            logs_bloom: format_fixed_bytes(receipt.inner.logs_bloom().0),
            root: receipt.state_root.map(format_fixed_bytes),
            status: Some(receipt.status().into()),
        })
    }

    Ok(vec)
}

pub async fn transform_traces<T: DebugExtraction>(
    extracted: &T,
    incl_traces: bool,
    incl_count: bool,
    incl_pertx_count: bool,
) -> Result<
    (
        Option<Vec<Trace>>,
        Option<usize>,
        Option<HashMap<Option<u64>, u64>>,
    ),
    ExtractTransformErr,
> {
    let traces = extracted.traces();

    let count: Option<usize> = {
        if incl_count {
            Some(traces.len())
        } else {
            None
        }
    };

    let mut pertx: Option<HashMap<Option<u64>, u64>> = {
        if incl_pertx_count {
            Some(HashMap::with_capacity(traces.len()))
        } else {
            None
        }
    };

    let mut trace_records_vec: Option<Vec<Trace>> = {
        if incl_traces {
            Some(Vec::with_capacity(traces.len()))
        } else {
            None
        }
    };

    // Iterate through the traces
    for (trace_index, trace) in traces.iter().enumerate() {
        // If storing the pertx hashmap, we add it
        if let Some(txmap) = &mut pertx {
            let (_, txpos) = trace.get_tx_info();
            match txmap.get_mut(&txpos) {
                Some(value) => *value += 1,
                None => {
                    txmap.insert(txpos, 1);
                }
            }
        }

        // If storing trace records, add them here
        if let Some(records) = &mut trace_records_vec {
            let trace_out = match trace {
                TxTrace::Call(call) => Trace {
                    block_hash: format!("{}", call.block_hash),
                    block_number: extracted.block_number(),
                    block_timestamp: extracted.block_timestamp() as u64,
                    transaction_hash: Some(format!("{}", call.transaction_hash)),
                    transaction_index: Some(call.transaction_position as i64),
                    trace_type: "call".to_string(),
                    trace_address: call
                        .trace_address
                        .iter()
                        .map(|addr| *addr as i64)
                        .collect::<Vec<i64>>(),
                    subtrace_count: call.subtraces as i64,
                    action: TraceAction {
                        from_address: Some(format!("{}", call.action.from)),
                        to_address: Some(format!("{}", call.action.to)),
                        call_type: Some(call.action.call_type.clone()),
                        gas: Some(call.action.gas as i64),
                        input: Some(call.action.input.clone()),
                        value: match U256::from_str_radix(
                            call.action
                                .value
                                .strip_prefix("0x")
                                .unwrap_or(&call.action.value),
                            16,
                        ) {
                            Ok(num) => Some(cap_bignumeric(&num)),
                            Err(err) => panic!("Failed to parse number: {}", err),
                        },
                        value_lossless: match U256::from_str_radix(
                            call.action
                                .value
                                .strip_prefix("0x")
                                .unwrap_or(&call.action.value),
                            16,
                        ) {
                            Ok(num) => Some(num.to_string()),
                            Err(err) => panic!("Failed to parse number: {}", err),
                        },
                        init: None,
                        author: None,
                        reward_type: None,
                        refund_address: None,
                        refund_balance: None,
                        refund_balance_lossless: None,
                        self_destructed_address: None,
                    },
                    result: call.result.as_ref().map(|res| TraceResult {
                        gas_used: match i64::try_from(res.gas_used) {
                            Ok(gas_used) => Some(gas_used),
                            Err(err) => panic!("Failed to convert gas_used to i64: {err}"),
                        },
                        output: Some(res.output.clone()),
                        address: None,
                        code: None,
                    }),
                    error: call.error.clone(),
                    trace_index: trace_index as u64,
                },
                TxTrace::Reward(rwd) => Trace {
                    block_hash: format!("{}", rwd.block_hash),
                    block_number: extracted.block_number(),
                    block_timestamp: extracted.block_timestamp() as u64,
                    transaction_hash: None,
                    transaction_index: None,
                    trace_type: "reward".to_string(),
                    trace_address: rwd
                        .trace_address
                        .iter()
                        .map(|item| *item as i64)
                        .collect::<Vec<i64>>(),
                    subtrace_count: rwd.subtraces as i64,
                    action: TraceAction {
                        from_address: None,
                        to_address: None,
                        call_type: None,
                        gas: None,
                        input: None,
                        value: Some(rwd.action.value.to_string()),
                        value_lossless: Some(rwd.action.value.to_string()),
                        reward_type: Some(rwd.action.reward_type.clone()),
                        init: None,
                        author: Some(format!("{}", rwd.action.author)),
                        refund_address: None,
                        refund_balance: None,
                        refund_balance_lossless: None,
                        self_destructed_address: None,
                    },
                    result: rwd.result.as_ref().map(|res| TraceResult {
                        gas_used: Some(res.gas_used as i64),
                        output: Some(res.output.clone()),
                        address: None,
                        code: None,
                    }),
                    error: rwd.error.clone(),
                    trace_index: trace_index as u64,
                },
                TxTrace::Suicide(suicide) => Trace {
                    block_hash: format!("{}", suicide.block_hash),
                    block_number: extracted.block_number(),
                    block_timestamp: extracted.block_timestamp() as u64,
                    transaction_hash: Some(format!("{}", suicide.transaction_hash)),
                    transaction_index: Some(suicide.transaction_position as i64),
                    trace_type: "suicide".to_string(),
                    trace_address: suicide
                        .trace_address
                        .iter()
                        .map(|x| *x as i64)
                        .collect::<Vec<i64>>(),
                    subtrace_count: suicide.subtraces as i64,
                    action: TraceAction {
                        from_address: None,
                        to_address: None,
                        call_type: None,
                        gas: None,
                        input: None,
                        value: None,
                        value_lossless: None,
                        init: None,
                        author: None,
                        reward_type: None,
                        refund_address: suicide.action.refund_address.map(|x| format!("{}", x)),
                        refund_balance: Some(suicide.action.balance.to_string()),
                        refund_balance_lossless: Some(suicide.action.balance.to_string()),
                        self_destructed_address: Some(format!(
                            "{}",
                            suicide.action.self_destructed_address
                        )),
                    },
                    result: None,
                    error: suicide.error.clone(),
                    trace_index: trace_index as u64,
                },
                TxTrace::Create(create) => Trace {
                    block_hash: format!("{}", create.block_hash),
                    block_number: extracted.block_number(),
                    block_timestamp: extracted.block_timestamp() as u64,
                    transaction_hash: Some(format!("{}", create.transaction_hash)),
                    transaction_index: Some(create.transaction_position as i64),
                    trace_type: "create".to_string(),
                    trace_address: Vec::new(),
                    subtrace_count: create.subtraces as i64,
                    action: TraceAction {
                        from_address: Some(format!("{}", create.action.from)),
                        to_address: None,
                        call_type: None,
                        gas: Some(create.action.gas as i64),
                        input: None,
                        value: Some(create.action.value.to_string()),
                        value_lossless: Some(create.action.value.to_string()),
                        init: Some(create.action.init.clone()),
                        author: None,
                        reward_type: None,
                        refund_address: None,
                        refund_balance: None,
                        refund_balance_lossless: None,
                        self_destructed_address: None,
                    },
                    result: create.result.as_ref().map(|res| TraceResult {
                        gas_used: Some(res.gas_used as i64),
                        output: None,
                        address: Some(format!("{}", res.address)),
                        code: Some(res.code.clone()),
                    }),
                    error: create.error.clone(),
                    trace_index: trace_index as u64,
                },
                TxTrace::Empty(empty) => Trace {
                    block_hash: format!("{}", empty.block_hash),
                    block_number: extracted.block_number(),
                    block_timestamp: extracted.block_timestamp() as u64,
                    transaction_hash: Some(format!("{}", empty.transaction_hash)),
                    transaction_index: Some(empty.transaction_position as i64),
                    trace_type: "empty".to_string(),
                    trace_address: Vec::new(),
                    subtrace_count: empty.subtraces as i64,
                    action: TraceAction {
                        from_address: Some(format!("{}", empty.action.from)),
                        to_address: None,
                        call_type: None,
                        gas: Some(empty.action.gas as i64),
                        input: None,
                        value: Some(empty.action.value.to_string()),
                        value_lossless: Some(empty.action.value.to_string()),
                        init: Some(empty.action.init.clone()),
                        author: None,
                        reward_type: None,
                        refund_address: None,
                        refund_balance: None,
                        refund_balance_lossless: None,
                        self_destructed_address: None,
                    },
                    result: empty.result.as_ref().map(|res| TraceResult {
                        gas_used: Some(res.gas_used as i64),
                        output: None,
                        address: None,
                        code: None,
                    }),
                    error: empty.error.clone(),
                    trace_index: trace_index as u64,
                },
            };
            records.push(trace_out);
        }
    }

    Ok((trace_records_vec, count, pertx))
}
