use std::sync::Arc;

use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties, options::BasicPublishOptions,
    publisher_confirm::Confirmation, types::FieldTable,
};
use mongodb::{bson::Document, change_stream::event::ChangeStreamEvent};
use mrcon_config::{Exchange, RabbitMq};
use mrcon_core::{Error, Publish};
use serde_json;
use tracing::trace;

/// RabbitMQ publisher for MongoDB change events.
///
/// The `Publisher` encapsulates a RabbitMQ channel and configuration, providing methods to declare queues
/// and publish MongoDB change stream events as JSON messages. It is used by the connector to forward
/// change events to RabbitMQ reliably.
pub struct Publisher {
    config: RabbitMq,
    channel: Channel,
    routing_key: String,
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
        Self::init(config.clone(), Arc::new(conn)).await
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
        Self::init(config, connection).await
    }

    /// Initialize a new `Publisher` with the given configuration and connection.
    ///
    /// This private method contains the common initialization logic for both `new` and `with_connection`.
    /// It creates a channel from the connection, declares the exchange (if specified), and declares the target queue.
    ///
    /// # Arguments
    /// * `config` - RabbitMQ configuration (queue/stream name, etc).
    /// * `connection` - An Arc-wrapped RabbitMQ `Connection`.
    ///
    /// # Errors
    /// Returns an error if the channel creation, exchange declaration, or queue declaration fails.
    async fn init(config: RabbitMq, connection: Arc<Connection>) -> Result<Self, Error> {
        let channel = connection.create_channel().await?;

        let routing_key = config
            .routing_key
            .clone()
            .unwrap_or_else(|| config.queue_name.clone());

        if config.exchange != Exchange::default() {
            // Declare exchange if exchange_name is specified
            channel
                .exchange_declare(
                    &config.exchange.name,
                    config.exchange.kind.clone().into(),
                    config.exchange.declare_options.clone().into(),
                    FieldTable::default(),
                )
                .await?;

            channel
                .queue_bind(
                    &config.queue.name,
                    &config.exchange.name,
                    &routing_key,
                    config.queue.bind_options.clone().into(),
                    FieldTable::default(),
                )
                .await?;
        }

        channel
            .queue_declare(
                &config.queue.name,
                config.queue.declare_options.clone().into(),
                FieldTable::default(),
            )
            .await?;
        Ok(Self {
            config,
            channel,
            routing_key,
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
                self.config.exchange.name.as_str(),
                self.routing_key.as_str(),
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?
            .await?;
        trace!(queue = %self.config.queue_name, "Published message to RabbitMQ, payload: {}, confirmation: {:?}", serde_json::to_string(event)?, confirm);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Publish for Publisher {
    async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error> {
        self.publish(event).await
    }
}
