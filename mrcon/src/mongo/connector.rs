use std::sync::Arc;

use crate::mongo::resume_tokens::ResumeTokensDB;
use crate::rabbitmq::Publisher;
use crate::{config::WatchedDb, rabbitmq::amqp};
use futures_util::stream::StreamExt;
use mongodb::{bson::Document, Client};
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum Error {
    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),
    #[error("RabbitMq error: {0}")]
    RabbitMq(#[from] crate::rabbitmq::Error),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Connector for streaming MongoDB change events to RabbitMQ with persistent resume tokens.
///
/// The `Connector` encapsulates all state and logic required to watch a MongoDB collection for change events,
/// publish those events to RabbitMQ, and persist resume tokens for reliable, resumable streaming. It is designed
/// to be constructed per watched collection and manages its own MongoDB client, resume token storage, and publisher.
pub struct Connector {
    client: Client,
    watched: WatchedDb,
    resume_tokens: ResumeTokensDB,
    publisher: Publisher,
}

impl Connector {
    /// Constructs a new `Connector` for a watched MongoDB collection and RabbitMQ publisher.
    ///
    /// This method initializes the MongoDB client, persistent resume token storage, and RabbitMQ publisher
    /// for the specified collection configuration. It is the main entry point for setting up a connector
    /// for a single collection, using the provided MongoDB and RabbitMQ URIs and collection settings.
    ///
    /// # Arguments
    ///
    /// * `mongo_uri` - MongoDB connection string.
    /// * `rabbitmq_uri` - RabbitMQ connection string.
    /// * `settings` - Collection-specific configuration, including watched collection, resume token storage, and RabbitMQ settings.
    ///
    /// # Errors
    ///
    /// Returns an error if the MongoDB client, resume token storage, or RabbitMQ publisher cannot be initialized.
    pub async fn from_collection(
        mongo_uri: &str,
        rabbitmq_uri: &str,
        settings: &crate::config::Collection,
    ) -> Result<Self, Error> {
        let client = Client::with_uri_str(mongo_uri).await?;

        let resume_tokens =
            ResumeTokensDB::new(client.clone(), settings.resume_tokens.clone()).await?;

        let amqp_publisher = amqp::Publisher::new(&settings.rabbitmq, rabbitmq_uri).await?;
        let publisher = Publisher::new(Arc::new(amqp_publisher));

        Self::new(client, settings.watched.clone(), resume_tokens, publisher).await
    }

    pub async fn with_clients(
        client: Client,
        rabbitmq_client: Arc<lapin::Connection>,
        settings: &crate::config::Collection,
    ) -> Result<Self, Error> {
        let resume_tokens =
            ResumeTokensDB::new(client.clone(), settings.resume_tokens.clone()).await?;

        let amqp_publisher =
            amqp::Publisher::with_connection(settings.rabbitmq.clone(), rabbitmq_client).await?;
        let publisher = Publisher::new(Arc::new(amqp_publisher));

        Self::new(client, settings.watched.clone(), resume_tokens, publisher).await
    }

    /// Creates a new `Connector` instance from its components.
    ///
    /// This is a lower-level constructor for advanced use cases, allowing direct injection of the MongoDB client,
    /// watched collection info, resume token storage, and RabbitMQ publisher.
    ///
    /// # Arguments
    ///
    /// * `client` - Initialized MongoDB client.
    /// * `watched` - Watched collection configuration.
    /// * `resume_tokens` - Persistent resume token storage.
    /// * `publisher` - RabbitMQ publisher abstraction.
    ///
    /// # Errors
    ///
    /// Returns an error if construction fails (should be infallible in most cases).
    pub async fn new(
        client: Client,
        watched: WatchedDb,
        resume_tokens: ResumeTokensDB,
        publisher: Publisher,
    ) -> Result<Self, Error> {
        Ok(Self {
            client,
            watched,
            resume_tokens,
            publisher,
        })
    }

    /// Streams MongoDB change events to RabbitMQ and persists resume tokens.
    ///
    /// This method opens a change stream on the configured MongoDB collection, optionally resuming from the last
    /// persisted resume token for the given `stream_name`. For each change event, it publishes the event to RabbitMQ
    /// and updates the resume token in persistent storage. If the collection is dropped, the resume tokens are cleaned up.
    ///
    /// # Arguments
    ///
    /// * `stream_name` - A unique identifier for the change stream, used for resume token persistence.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a failure in reading from MongoDB, publishing to RabbitMQ, or persisting resume tokens.
    pub async fn connect(&self, stream_name: &str) -> Result<(), Error> {
        let collection = self
            .client
            .database(&self.watched.db_name)
            .collection::<Document>(&self.watched.coll_name);

        debug!(db = %self.watched.db_name, coll = %self.watched.coll_name, "Watching collection");

        let resume_token = self
            .resume_tokens
            .get_last_resume_token(stream_name)
            .await?;

        debug!(db = %self.watched.db_name, "Watching collection, resume token: {:?}", resume_token);

        let mut change_stream = collection.watch().resume_after(resume_token).await?;
        info!(db = %self.watched.db_name, coll = %self.watched.coll_name, "Started watching collection");

        while let Some(change) = change_stream.next().await.transpose().map_err(|e| {
            error!(error = %e, "Change stream error");
            e
        })? {
            debug!(db = %self.watched.db_name, coll = %self.watched.coll_name, "Received change event: {:?}", change);
            self.publisher.publish(&change).await.map_err(|e| {
                error!(error = %e, "Failed to publish change event to RabbitMQ");
                e
            })?;

            if let Some(token) = change_stream.resume_token() {
                self.resume_tokens
                    .set_last_resume_token(stream_name, &token)
                    .await
                    .map_err(|e| {
                        error!(error = %e, "Failed to save resume token");
                        e
                    })?;
                debug!("Saved resume token: {}", serde_json::to_string(&token)?);
            } else {
                warn!("No resume token found in change event");
            }
        }

        warn!("Collection dropped, stopping watcher, dropping resume tokens");
        self.resume_tokens.clean().await?;

        Ok(())
    }
}
