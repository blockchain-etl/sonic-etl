//! This module contains implementation details for
//! StreamPublisherConnection when the `GOOGLE_PUBSUB`
//! feature is enabled.  This allows StreamPublisherConnection
//! to connect and publish to the GCP's PubSub service.
use log::info;
use log::warn;
use std::time;
use tokio::time::sleep;

#[cfg(feature = "APACHE_AVRO")]
use crate::blockchain_config::avro_helpers::{env_key_to_table_name, table_to_avro};

use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    publisher::Publisher,
};

use prost::Message;

use super::environment::*;
use super::publish::{StreamPublisherConnection, StreamPublisherConnectionClient};

/// Establishes the connection to the Google Cloud Pub/Sub extracting the credentials
/// and information from the .env file.  This function creates the connection for
/// using a single publisher.
/// Must have the `GCP_CREDENTIAL_JSON_PATH` filepath pointing to the credentials json file,
/// and have `GOOGLE_PUBSUB_TOPIC` string saved in the .env file.
pub async fn connect(queue_name: &str) -> StreamPublisherConnection {
    let gcp_config = {
        match get_gcp_credentials_json_path() {
            Some(key_path) => {
                let cred_file = CredentialsFile::new_from_file(key_path.to_owned())
                    .await
                    .expect("GCP credentials file exists");
                // authenticate using the key file
                ClientConfig::default()
                    .with_credentials(cred_file)
                    .await
                    .unwrap()
            }
            None => ClientConfig::default().with_auth().await.unwrap(),
        }
    };

    // Attempt to create the client using the configuration from above
    let gcp_client = Client::new(gcp_config).await.unwrap();

    // Use the client to connect to the specific topic.
    connect_to_topic(gcp_client.clone(), queue_name).await
}

/// Establishes a connection to the Google Cloud Pub/Sub Topic.  Assumes that the
/// pubsub topic has already been created in Google Cloud Platform (GCP), and panics
/// if the topic does not exist.
/// Should provide the GCP Client and the topic_name, where `topic_name` **is the name
/// of a property in the .env file**.  Not the actual topic name itself.
async fn connect_to_topic(
    gcp_client: google_cloud_pubsub::client::Client,
    topic_name: &str,
) -> StreamPublisherConnection {
    // TODO: the name of the avro schema file may be retrievable from the `topic_name` argument, but keep in mind that `topic name` is the env var (like all caps)
    // maybe we should have some sort of macro or enum to map from table->topic ENV var, or table->schema.
    #[cfg(feature = "APACHE_AVRO")]
    let avro_schema = {
        let table_name = env_key_to_table_name(topic_name);
        let avro_schema_str = table_to_avro(table_name);
        apache_avro::Schema::parse_str(avro_schema_str).unwrap()
    };

    let google_pubsub_topic = dotenvy::var(topic_name)
        .expect("GOOGLE_PUBSUB_TOPIC should exist in .env file")
        .parse::<String>()
        .unwrap();

    // NOTE: assumes that this pubsub topic has already been created in GCP.
    let topic = gcp_client.topic(&google_pubsub_topic);

    if !topic.exists(None).await.unwrap() {
        panic!(
            "Topic {} doesn't exist! Terminating...",
            google_pubsub_topic
        );
    } else {
        info!("Topic exists. Proceeding...");
    }
    let publisher = topic.new_publisher(None);
    StreamPublisherConnection {
        client: StreamPublisherConnectionClient::GcpPubSub(publisher),
        queue_name: topic_name.to_string(),
        #[cfg(feature = "APACHE_AVRO")]
        schema: avro_schema,
    }
}

/// creates a PubsubMessage object using the bytes
fn prepare_message(serialized_block: Vec<u8>) -> PubsubMessage {
    PubsubMessage {
        data: serialized_block,
        ..Default::default()
    }
}

#[allow(non_snake_case)]
impl StreamPublisherConnectionClient {
    /// Sends a message to a Google Pub/Sub topic
    pub async fn publish(&self, msg: Vec<u8>) {
        let StreamPublisherConnectionClient::GcpPubSub(Publisher) = self;
        // publish the message
        let prepared_msg = prepare_message(msg);
        publish_with_backoff(Publisher, prepared_msg).await;
    }

    /// Sends a batch of messages to a Google Pub/Sub topic
    pub async fn publish_batch(&self, msg_batch: Vec<Vec<u8>>) {
        let StreamPublisherConnectionClient::GcpPubSub(Publisher) = self;
        // publish the message batch

        let prepared_msgs: Vec<PubsubMessage> =
            msg_batch.into_iter().map(prepare_message).collect();
        let message_chunks = prepared_msgs.chunks(900);
        for chunk in message_chunks.into_iter() {
            publish_batch_with_backoff(Publisher, chunk.to_vec()).await;
        }
    }

    pub async fn disconnect(&mut self) {
        let StreamPublisherConnectionClient::GcpPubSub(Publisher) = self;
        let gcp_publisher = Publisher;
        gcp_publisher.shutdown().await;
    }
}

/// Publishes a message to google cloud pub/sub.
/// Each time publishing fails, the sleep time is increased by 1 second.
async fn publish_with_backoff(publisher: &Publisher, message: PubsubMessage) {
    let awaiter = publisher.publish(message.clone()).await;
    let mut res = awaiter.get().await;
    let mut backoff = 0;
    loop {
        info!("Message publish result: {:?}", res);
        match res {
            Ok(_) => break,
            Err(_) => {
                warn!("publish failed for publisher: {:?}", publisher);
                let seconds = time::Duration::from_secs(backoff);
                sleep(seconds).await;
                backoff += 1;
                let awaiter = publisher.publish(message.clone()).await;
                res = awaiter.get().await;
            }
        }
    }
}

/// Attempts to publish a batch of messages to google cloud pub/sub.
/// If publishing fails, each individual message is published separately.
async fn publish_batch_with_backoff(publisher: &Publisher, messages: Vec<PubsubMessage>) {
    let awaiters = publisher.publish_bulk(messages.clone()).await;
    for (i, awaiter) in awaiters.into_iter().enumerate() {
        let res = awaiter.get().await;
        match res {
            Err(_) => {
                let msg = messages[i].clone();
                publish_with_backoff(publisher, msg).await;
            }
            Ok(_) => continue,
        }
    }
}

impl StreamPublisherConnection {
    /// Publish the message to Pub/Sub as an Apache Avro message.
    #[cfg(feature = "APACHE_AVRO")]
    pub async fn publish<T: serde::Serialize>(&self, msg: T) {
        let mut writer = apache_avro::Writer::new(&self.schema, Vec::new());
        writer
            .append_ser(msg)
            .expect("protobuf schema matches avro schema");
        let encoded = writer.into_inner().unwrap();

        self.client.publish(encoded).await;
    }

    /// Publish the message to Pub/Sub as a Protocol Buffers message.
    #[cfg(not(feature = "APACHE_AVRO"))]
    pub async fn publish<T: Message>(&self, msg: T) {
        self.client.publish(msg.encode_to_vec()).await;
    }
    /// Sends the messages to the client
    pub async fn publish_batch<T: Message>(&self, msgs: Vec<T>) {
        self.client
            .publish_batch(msgs.iter().map(|msg| msg.encode_to_vec()).collect())
            .await;
    }

    pub async fn disconnect(mut self) {
        self.client.disconnect().await;
    }
}
