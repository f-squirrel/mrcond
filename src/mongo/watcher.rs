use crate::config::WatchedDb;
use crate::mongo::resume_tokens::ResumeTokensDB;
use crate::rabbitmq::Publisher;
use bson;
use futures_util::stream::StreamExt;
use mongodb::{bson::Document, Client};
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum Error {
    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),
    #[error("Publisher error: {0}")]
    Publisher(#[from] crate::rabbitmq::Error),
}

pub struct Watcher {
    client: Client,
    watched: WatchedDb,
    resume_tokens: ResumeTokensDB,
    publisher: Publisher,
}

impl Watcher {
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

    pub async fn watch(&self, stream_name: &str) -> Result<(), Error> {
        let collection = self
            .client
            .database(&self.watched.db_name)
            .collection::<Document>(&self.watched.coll_name);

        let resume_token = self
            .resume_tokens
            .get_last_resume_token(stream_name)
            .await?
            .and_then(|b| bson::from_bson(b).ok());

        let mut change_stream = collection.watch().resume_after(resume_token).await?;
        info!(db = %self.watched.db_name, coll = %self.watched.coll_name, "Started watching collection");

        while let Some(change) = change_stream.next().await.transpose().map_err(|e| {
            error!(error = %e, "Change stream error");
            e
        })? {
            self.publisher.publish(&change).await.map_err(|e| {
                error!(error = %e, "Failed to publish change event to RabbitMQ");
                e
            })?;

            if let Some(token) = change_stream.resume_token() {
                self.resume_tokens
                    .set_last_resume_token(stream_name, token)
                    .await
                    .map_err(|e| {
                        error!(error = %e, "Failed to save resume token");
                        e
                    })?;
            } else {
                warn!("No resume token found in change event");
            }
        }

        // TODO(DD): Add an error saying that the collection was dropped
        Ok(())
    }
}
