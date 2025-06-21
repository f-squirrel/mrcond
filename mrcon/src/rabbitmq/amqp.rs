use std::sync::Arc;

use super::error::Error;
use super::publish::Publish;
use crate::config::RabbitMq;
use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties, options::BasicPublishOptions,
    publisher_confirm::Confirmation, types::FieldTable,
};
use mongodb::{bson::Document, change_stream::event::ChangeStreamEvent};
use serde_json;
use tracing::trace;

/// RabbitMQ publisher for MongoDB change events.
///
/// The `Publisher` encapsulates a RabbitMQ channel and configuration, providing methods to declare queues
/// and publish MongoDB change stream events as JSON messages. It is used by the connector to forward
/// change events to RabbitMQ reliably.
pub struct Publisher {
    pub config: RabbitMq,
    channel: Channel,
    // DD: to hold a connection while the channel is alive
    _connection: Arc<Connection>,
}

impl Publisher {
    /// Create a new `Publisher` for the given RabbitMQ configuration and URI.
    ///
    /// This method establishes a connection to RabbitMQ, creates a channel, and declares the target queue.
    ///
    /// # Arguments
    /// * `config` - RabbitMQ configuration (queue/stream name, etc).
    /// * `rabbitmq_uri` - Connection string for RabbitMQ.
    ///
    /// # Errors
    /// Returns an error if the connection, channel, or queue declaration fails.
    pub async fn new(config: &RabbitMq, rabbitmq_uri: &str) -> Result<Self, Error> {
        let conn = Connection::connect(rabbitmq_uri, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;
        channel
            .queue_declare(
                &config.stream_name,
                Default::default(),
                FieldTable::default(),
            )
            .await?;
        Ok(Self {
            config: config.clone(),
            channel,
            _connection: Arc::new(conn),
        })
    }

    /// Create a new `Publisher` using an existing RabbitMQ connection.
    ///
    /// This is the preferred way to construct a `Publisher` when you plan to create
    /// multiple publishers that share the same RabbitMQ connection (for example,
    /// when you want to avoid opening a new TCP connection for each publisher).
    ///
    /// # Arguments
    /// * `config` - RabbitMQ configuration (queue/stream name, etc).
    /// * `connection` - An existing, shared RabbitMQ `Connection`.
    ///
    /// # Returns
    /// Returns a new `Publisher` instance with its own channel, but sharing the provided connection.
    ///
    /// # Errors
    /// Returns an error if the channel creation or queue declaration fails.
    pub async fn with_connection(
        config: RabbitMq,
        connection: Arc<Connection>,
    ) -> Result<Self, Error> {
        let channel = connection.create_channel().await?;
        channel
            .queue_declare(
                &config.stream_name,
                Default::default(),
                FieldTable::default(),
            )
            .await?;
        Ok(Self {
            config,
            channel,
            _connection: connection,
        })
    }

    /// Publish a MongoDB change event to RabbitMQ as a JSON message.
    ///
    /// # Arguments
    /// * `event` - The MongoDB change stream event to publish.
    ///
    /// # Errors
    /// Returns an error if serialization or publishing fails.
    pub async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error> {
        let payload = serde_json::to_vec(event)?;
        let confirm: Confirmation = self
            .channel
            .basic_publish(
                "",
                &self.config.stream_name,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?
            .await?;
        trace!(queue = %self.config.stream_name, "Published message to RabbitMQ, payload: {}, confirmation: {:?}", serde_json::to_string(event)?, confirm);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Publish for Publisher {
    async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error> {
        self.publish(event).await
    }
}
