use crate::config::WatchedDb;
use crate::mongo::resume_tokens::ResumeTokensDB;
use bson;
use futures_util::stream::StreamExt;
use mongodb::{bson::Document, Client};
use tracing::{debug, error, info};

pub struct Watcher {
    pub client: Client,
    pub watched: WatchedDb,
    pub resume_tokens: ResumeTokensDB,
}

impl Watcher {
    pub async fn new(
        client: Client,
        watched: WatchedDb,
        resume_tokens: ResumeTokensDB,
    ) -> mongodb::error::Result<Self> {
        Ok(Self {
            client,
            watched,
            resume_tokens,
        })
    }

    pub async fn watch(&self, stream_name: &str) -> mongodb::error::Result<()> {
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
        while let Some(event) = change_stream.next().await {
            match event {
                Ok(change) => {
                    debug!(?change, "Change event");

                    if let Some(token) = change_stream.resume_token() {
                        if let Err(e) = self
                            .resume_tokens
                            .set_last_resume_token(stream_name, bson::to_bson(&token)?)
                            .await
                        {
                            error!(error = %e, "Failed to save resume token");
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Change stream error");
                    break;
                }
            }
        }
        Ok(())
    }
}
