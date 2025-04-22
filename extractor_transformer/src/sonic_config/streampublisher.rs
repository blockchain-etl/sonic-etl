//! This file contains the streampublisher, a struct containing all the StreamPublisherConnections.
//! This is specific to blockchains since we have different outputs per blockchain.

// Conditional imports
use crate::{self as blockchain_generic};

use blockchain_generic::output::publish::StreamPublisherConnection;
use log::info;

#[cfg(feature = "APACHE_KAFKA")]
use blockchain_generic::output::apache_kafka::connect;
#[cfg(feature = "GOOGLE_CLOUD_STORAGE")]
use blockchain_generic::output::gcs::connect;
#[cfg(feature = "GOOGLE_PUBSUB")]
use blockchain_generic::output::google_pubsub::connect;
#[cfg(feature = "JSON")]
use blockchain_generic::output::json::connect;
#[cfg(feature = "JSONL")]
use blockchain_generic::output::jsonl::{connect, connect_customdir};
#[cfg(feature = "RABBITMQ_CLASSIC")]
use blockchain_generic::output::rabbitmq_classic::connect;
#[cfg(feature = "RABBITMQ_STREAM")]
use blockchain_generic::output::rabbitmq_stream::connect;

/// StreamPublisher struct (seperate-publisher version) that contains various output
/// streams for different content.
#[cfg(feature = "SEPARATE_PUBLISHERS")]
#[derive(Clone)]
pub struct StreamPublisher {
    pub blocks: StreamPublisherConnection,
    pub decoded_events: StreamPublisherConnection,
    pub logs: StreamPublisherConnection,
    pub receipts: StreamPublisherConnection,
    pub transactions: StreamPublisherConnection,
    pub traces: StreamPublisherConnection
}

#[cfg(feature = "SEPARATE_PUBLISHERS")]
impl StreamPublisher {
    #[cfg(feature = "APACHE_KAFKA")]
    pub async fn with_producer(self) -> StreamPublisher {
        info!("Construction kafka producers...");
        StreamPublisher {
            blocks: self.blocks.with_producer().await,
            decoded_events: self.decoded_events.with_producer().await,
            logs: self.logs.with_producer().await,
            receipts: self.receipts.with_producer().await,
            transactions: self.transactions.with_producer().await,
            traces: self.traces.with_producer().await
        }
    }

    #[cfg(feature = "RABBITMQ_CLASSIC")]
    pub async fn with_channel(self) -> StreamPublisher {
        info!("Construction rabbitmq channgels...");
        StreamPublisher {
            blocks: self.blocks.with_channel().await,
            decoded_events: self.decoded_events.with_channel().await,
            logs: self.logs.with_channel().await,
            receipts: self.receipts.with_channel().await,
            transactions: self.transactions.with_channel().await,
            traces: self.traces.with_channel().await
        }
    }

    pub async fn new() -> StreamPublisher {
        info!("Connecting to the publishers...");
        StreamPublisher {
            blocks: connect("QUEUE_NAME_BLOCKS").await,
            decoded_events: connect("QUEUE_NAME_DECODED_EVENTS").await,
            logs: connect("QUEUE_NAME_LOGS").await,
            receipts: connect("QUEUE_NAME_RECEIPTS").await,
            transactions: connect("QUEUE_NAME_TRANSACTIONS").await,
            traces: connect("QUEUE_NAME_TRACES").await
        }
    }

    #[cfg(feature="PUBLISHER_CUSTOMDIR")]
    pub async fn new_customdir(dir: &str) -> StreamPublisher {
        StreamPublisher {
            blocks: connect_customdir(dir, "QUEUE_NAME_BLOCKS").await,
            decoded_events: connect_customdir(dir, "QUEUE_NAME_DECODED_EVENTS").await,
            logs: connect_customdir(dir, "QUEUE_NAME_LOGS").await,
            receipts: connect_customdir(dir, "QUEUE_NAME_RECEIPTS").await,
            transactions: connect_customdir(dir, "QUEUE_NAME_TRANSACTIONS").await,
            traces: connect_customdir(dir, "QUEUE_NAME_TRACES").await
        }
    }

    #[cfg(feature = "REQUIRES_DISCONNECT")]
    pub async fn disconnect(self) {
        info!("Disconnecting from publishers...");
        self.blocks.disconnect().await;
        self.decoded_events.disconnect().await;
        self.logs.disconnect().await;
        self.receipts.disconnect().await;
        self.transactions.disconnect().await;
        self.traces.disconnect().await;
    }
}
