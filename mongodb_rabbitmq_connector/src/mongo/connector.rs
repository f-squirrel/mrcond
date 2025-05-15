use std::sync::Arc;

use crate::mongo::resume_tokens::ResumeTokensDB;
use crate::rabbitmq::Publisher;
use crate::{config::WatchedDb, rabbitmq::amqp};
use bson;
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

pub struct Connector {
    client: Client,
    watched: WatchedDb,
    resume_tokens: ResumeTokensDB,
    publisher: Publisher,
}

impl Connector {
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

    pub async fn connect(&self, stream_name: &str) -> Result<(), Error> {
        let collection = self
            .client
            .database(&self.watched.db_name)
            .collection::<Document>(&self.watched.coll_name);

        debug!(db = %self.watched.db_name, coll = %self.watched.coll_name, "Watching collection");

        let resume_token = self
            .resume_tokens
            .get_last_resume_token(stream_name)
            .await?
            .and_then(|b| bson::from_bson(b).ok());

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

        // TODO(DD): Add an error saying that the collection was dropped
        Ok(())
    }
}
