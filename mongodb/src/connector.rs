use std::sync::Arc;

use crate::resume_tokens::ResumeTokensDB;
use futures_util::stream::StreamExt;
use mongodb::{Client, bson::Document};
use mrcon_config::WatchedDb;
use mrcon_core::{Error, Publish};
use tracing::{debug, error, info, warn};

/// Connector for streaming MongoDB change events to RabbitMQ with persistent resume tokens.
///
/// The `Connector` encapsulates all state and logic required to watch a MongoDB collection for change events,
/// publish those events to RabbitMQ, and persist resume tokens for reliable, resumable streaming. It is designed
/// to be constructed per watched collection and manages its own MongoDB client, resume token storage, and publisher.
pub struct Connector<P>
where
    P: Publish,
{
    client: Client,
    watched: WatchedDb,
    resume_tokens: ResumeTokensDB,
    publisher: Arc<P>,
}

impl<P> Connector<P>
where
    P: Publish,
{
    /// Creates a new `Connector` instance from its components.
    ///
    /// Lower-level constructor for advanced use cases, allowing direct injection of the MongoDB client,
    /// watched collection info, resume token storage, and RabbitMQ publisher.
    ///
    /// # Arguments
    /// * `client` - Initialized MongoDB client.
    /// * `watched` - Watched collection configuration.
    /// * `resume_tokens` - Persistent resume token storage.
    /// * `publisher` - RabbitMQ publisher abstraction.
    ///
    /// # Errors
    /// Returns an error if construction fails (should be infallible in most cases).
    pub async fn new(
        client: Client,
        watched: WatchedDb,
        resume_tokens: ResumeTokensDB,
        publisher: Arc<P>,
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
    /// Opens a change stream on the configured MongoDB collection, optionally resuming from the last
    /// persisted resume token for the given `stream_name`. For each change event, publishes the event to RabbitMQ
    /// and updates the resume token in persistent storage. If the collection is dropped, the resume tokens are cleaned up.
    ///
    /// # Arguments
    /// * `stream_name` - A unique identifier for the change stream, used for resume token persistence.
    ///
    /// # Errors
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
                        Error::MongoDB(e)
                    })?;
                debug!("Saved resume token: {}", serde_json::to_string(&token)?);
            } else {
                warn!("No resume token found in change event");
            }
        }

        warn!("Collection dropped, stopping watcher, dropping resume tokens");
        self.resume_tokens.clean().await.map_err(Error::MongoDB)?;

        Ok(())
    }
}
