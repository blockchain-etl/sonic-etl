use crate::blockchain_config::proto_codegen::etl::{simprequest::SimpleIndexingRequest, request::IndexingRequest};

impl From<IndexingRequest> for SimpleIndexingRequest {
    fn from(value: IndexingRequest) -> Self {
        Self {
            start: value.start,
            end: value.end
        }
    }
}

impl From<SimpleIndexingRequest> for IndexingRequest {
    fn from(value: SimpleIndexingRequest) -> Self {
        Self {
            start: value.start,
            end: value.end,
            ..Default::default()
        }
    }
}