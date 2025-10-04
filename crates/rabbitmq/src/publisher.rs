use std::sync::Arc;

use lapin::{
    Channel, Connection, ConnectionProperties, publisher_confirm::Confirmation, types::FieldTable,
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

        // Enable publisher confirmations for reliable message delivery
        channel
            .confirm_select(lapin::options::ConfirmSelectOptions::default())
            .await?;

        let routing_key = config
            .routing_key
            .clone()
            .unwrap_or_else(|| config.queue.name.clone());

        channel
            .queue_declare(
                &config.queue.name,
                config.queue.declare_options.into(),
                FieldTable::default(),
            )
            .await?;

        if config.exchange != Exchange::default() {
            tracing::debug!(
                exchange = %config.exchange.name,
                kind = %config.exchange.kind,
                "Declaring exchange"
            );
            // Declare exchange if exchange_name is specified
            channel
                .exchange_declare(
                    &config.exchange.name,
                    config.exchange.kind.clone().into(),
                    config.exchange.declare_options.into(),
                    FieldTable::default(),
                )
                .await?;

            tracing::debug!(
                queue = %config.queue.name,
                exchange = %config.exchange.name,
                routing_key = %routing_key,
                "Declaring and binding queue to exchange"
            );

            tracing::debug!(
                queue = %config.queue.name,
                exchange = %config.exchange.name,
                routing_key = %routing_key,
                "Binding queue to exchange"
            );

            channel
                .queue_bind(
                    &config.queue.name,
                    &config.exchange.name,
                    &routing_key,
                    config.queue.bind_options.into(),
                    FieldTable::default(),
                )
                .await?;
        }

        Ok(Self {
            config,
            channel,
            routing_key,
            _connection: connection,
        })
    }

    /// Publish a MongoDB change event to RabbitMQ as a JSON message with delivery confirmation.
    ///
    /// This method uses RabbitMQ's publisher confirmation mechanism to ensure reliable message delivery.
    /// It will:
    /// 1. Serialize the event to JSON
    /// 2. Publish the message to RabbitMQ
    /// 3. Wait for broker confirmation (ACK/NACK)
    /// 4. Return an error if the message was rejected (NACK) or could not be routed
    ///
    /// For improved reliability, consider setting these configuration options:
    /// - `basic_publish_options.mandatory = true` - Get notified if message is unroutable
    /// - `basic_properties.delivery_mode = 2` - Make messages persistent
    /// - `queue.declare_options.durable = true` - Make queue survive broker restart
    ///
    /// # Arguments
    /// * `event` - The MongoDB change stream event to publish.
    ///
    /// # Errors
    /// Returns an error if:
    /// - JSON serialization fails
    /// - Network/connection issues occur
    /// - The broker rejects the message (NACK)
    /// - The message cannot be routed (when mandatory=true)
    pub async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error> {
        let payload = serde_json::to_vec(event)?;
        let confirm: Confirmation = self
            .channel
            .basic_publish(
                self.config.exchange.name.as_str(),
                self.routing_key.as_str(),
                self.config.queue.basic_publish_options.into(),
                &payload,
                self.config.queue.basic_properties.clone().into(),
            )
            .await?
            .await?;

        // Check if the publish was actually successful
        match confirm {
            Confirmation::Ack(_) => {
                trace!(queue = %self.config.queue.name, "Published message to RabbitMQ successfully, payload: {}", serde_json::to_string(event)?);
                Ok(())
            }
            Confirmation::Nack(nack) => {
                let error_msg = format!(
                    "Message was rejected by RabbitMQ broker (NACK received): {:?}",
                    nack
                );
                tracing::error!(queue = %self.config.queue.name, "{}", error_msg);
                Err(Error::RabbitMq(lapin::Error::from(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    error_msg,
                ))))
            }
            Confirmation::NotRequested => {
                // This shouldn't happen in our case since we're awaiting the confirmation
                tracing::trace!(queue = %self.config.queue.name, "Publisher confirmation was not requested");
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait]
impl Publish for Publisher {
    async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error> {
        self.publish(event).await
    }
}
