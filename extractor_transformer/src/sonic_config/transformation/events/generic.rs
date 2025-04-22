use alloy::{rpc::types::eth::Log, dyn_abi::{DecodedEvent, EventExt}, json_abi::Event, primitives::B256};



pub trait EventCatalog {
    

    /// Returns an [Event] given the signature as a [B256] (often the first topic of a log)
    /// 
    /// Should attempt to return the [Event], and if cannot, run [EventCatalog::get_event_by_signature]
    /// on the value returned by 
    fn get_event_by_signature_and_ntopics(&self, signature: &B256, n_topics: u8) -> Result<&Event, GetEventBySigErr>;

    /// Attempts to decode the provided log by searching for an applicable event in the [EventCatalog]
    fn attempt_decode_log(&self, log: &Log) -> Result<DecodedEventExt, LogDecodeErr> {
        match log.topics().first().map(|maybe_sig| self.get_event_by_signature_and_ntopics(maybe_sig, (log.topics().len()) as u8)) {
            Some(Ok(event)) => {
                match event.decode_log(log.data(), true).map_err(LogDecodeErr::DecodeErr) {
                    Ok(decoded) => Ok(DecodedEventExt {
                        event: event.clone(),
                        decoded
                    }),
                    Err(err) => Err(err),
                }
            },
            Some(Err(err)) => Err(LogDecodeErr::EventRetrievalErr(err)),
            None => Err(LogDecodeErr::LogHasNoTopics),
        }
    }

}
#[derive(Debug)]
pub enum GetEventBySigErr {
    NotFound
}

impl std::fmt::Display for GetEventBySigErr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event not found")
    }
}

impl std::error::Error for GetEventBySigErr {}

#[derive(Debug)]
pub enum LogDecodeErr {
    EventRetrievalErr(GetEventBySigErr),
    LogHasNoTopics,
    DecodeErr(alloy::dyn_abi::Error),
}

impl std::fmt::Display for LogDecodeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventRetrievalErr(err) => err.fmt(f),
            Self::LogHasNoTopics => write!(f, "Log has no topics to decode"),
            Self::DecodeErr(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for LogDecodeErr {}

#[derive(Debug, Clone)]
pub struct DecodedEventExt {
    pub event: Event,
    pub decoded: DecodedEvent,
}

use alloy::dyn_abi::DynSolValue;
use serde_json::Value;


#[inline]
pub fn solval_to_serdeval(value: &DynSolValue) -> Result<Value, SolvalToSerdevalErr> {
    match value {
        DynSolValue::Address(addr) => Ok(Value::String(addr.to_string())),
        DynSolValue::Bool(bool) => Ok(Value::Bool(*bool)),
        DynSolValue::String(string) => Ok(Value::String(string.clone())),
        DynSolValue::Array(vec) | DynSolValue::FixedArray(vec) | DynSolValue::Tuple(vec)=> {
            Ok(Value::Array(vec.iter().map(solval_to_serdeval).collect::<Result<Vec<_>,_>>()?))
        },
        DynSolValue::Bytes(bytes) => {
            Ok(Value::Array(bytes.iter().map(|byte| Value::from(*byte)).collect::<Vec<Value>>()))
        },
        DynSolValue::FixedBytes(word, size) => {
            match word.0.get(..*size) {
                Some(bytes) => Ok(Value::Array(bytes.iter().map(|byte| Value::from(*byte)).collect::<Vec<Value>>())),
                None => Err(SolvalToSerdevalErr::SizeExceeded32(*size))
            }
        },
        DynSolValue::Int(num, _) => Ok(Value::String(num.to_string())),
        DynSolValue::Uint(num, _) => Ok(Value::String(num.to_string())),
        _ => Err(SolvalToSerdevalErr::NotImpl)
    }
}

#[derive(Debug)]
pub enum SolvalToSerdevalErr {
    NotImpl,
    SizeExceeded32(usize)
}

impl DecodedEventExt {

    #[inline]
    pub fn args_to_json(&self) -> Result<serde_json::Value, ArgsToJsonErr> {

        let mut solvals = Vec::with_capacity(self.event.inputs.len());

        let mut n_indexed = 0_usize;
        let mut n_other = 0_usize;

        for param in self.event.inputs.iter() {
            match param.indexed {
                true => {
                    match self.decoded.indexed.get(n_indexed) {
                        Some(value) => solvals.push(solval_to_serdeval(value)?),
                        None => return Err(ArgsToJsonErr::MissingValue),
                    }
                    n_indexed += 1;
                },
                false => {
                    match self.decoded.body.get(n_other) {
                        Some(value) => solvals.push(solval_to_serdeval(value)?),
                        None => return Err(ArgsToJsonErr::MissingValue),
                    }
                    n_other += 1;
                },
            }
        }

        Ok(Value::Array(solvals))
    }

}

#[derive(Debug)]
pub enum ArgsToJsonErr {
    FromSolvalErr(SolvalToSerdevalErr),
    MissingValue
}

impl From<SolvalToSerdevalErr> for ArgsToJsonErr {
    #[inline]
    fn from(value: SolvalToSerdevalErr) -> Self {
        Self::FromSolvalErr(value)
    }
}


pub trait DynEventCatalog {

    fn add_event(&mut self, event: &Event) -> Result<(), AddEventErr>;

    fn pop_event(&mut self, signature: &B256, n_topics: u8) -> Result<Event, PopEventErr>;

    #[inline]
    fn remove_event(&mut self, signature: &B256, n_topics: u8) -> Result<(), PopEventErr> {
        self.pop_event(signature, n_topics).map(|_| ())
    }

}

#[derive(Debug)]
pub enum AddEventErr {
    AlreadyAdded,
}

#[derive(Debug)]
pub enum PopEventErr {
    DoesNotExist,
}