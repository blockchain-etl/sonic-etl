use alloy::{json_abi::Event, primitives::B256};

use super::generic::{EventCatalog, GetEventBySigErr, LogDecodeErr};

/// [NullEventCatalog] is an [EventCatalog] built to always fail.  Useful for a placeholder when 
/// an [EventCatalog] type is needed but not used.
pub struct NullEventCatalog();

impl EventCatalog for NullEventCatalog {

    #[inline]
    fn get_event_by_signature_and_ntopics(&self, _signature: &B256, _n_topics: u8) -> Result<&Event, GetEventBySigErr> {
        Err(GetEventBySigErr::NotFound)
    }

    #[inline(always)]
    fn attempt_decode_log(&self, _log: &alloy::rpc::types::Log) -> Result<super::generic::DecodedEventExt, LogDecodeErr> {
        Err(LogDecodeErr::EventRetrievalErr(GetEventBySigErr::NotFound))
    }
}