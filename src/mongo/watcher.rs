use crate::mongo::resume_tokens::ResumeTokens;
use bson;
use futures_util::stream::StreamExt;
use mongodb::{bson::Document, change_stream::event::ResumeToken, Client};
use tracing::{debug, error, info};

pub struct Watcher {
    pub client: Client,
    pub db_name: String,
    pub coll_name: String,
    pub change_stream_pre_and_post_images: bool,
    pub resume_token: Option<ResumeToken>,
    pub resume_tokens: ResumeTokens,
}

impl Watcher {
    pub async fn new(
        client: Client,
        db_name: String,
        coll_name: String,
        change_stream_pre_and_post_images: bool,
        resume_tokens: ResumeTokens,
        stream_name: &str,
    ) -> mongodb::error::Result<Self> {
        // Try to get the last resume token from storage
        let token_bson = resume_tokens.get_last_resume_token(stream_name).await?;
        let resume_token = token_bson.and_then(|b| bson::from_bson(b).ok());
        Ok(Self {
            client,
            db_name,
            coll_name,
            change_stream_pre_and_post_images,
            resume_token,
            resume_tokens,
        })
    }

    pub async fn watch(&mut self, stream_name: &str) -> mongodb::error::Result<()> {
        let collection = self
            .client
            .database(&self.db_name)
            .collection::<Document>(&self.coll_name);
        let mut change_stream = collection
            .watch()
            .resume_after(self.resume_token.clone())
            .await?;
        info!(db = %self.db_name, coll = %self.coll_name, "Started watching collection");
        while let Some(event) = change_stream.next().await {
            match event {
                Ok(change) => {
                    let resume_token = change_stream.resume_token();
                    debug!(?change, "Change event");
                    // Save the new resume token
                    if let Some(token) = resume_token {
                        if let Err(e) = self
                            .resume_tokens
                            .set_last_resume_token(stream_name, bson::to_bson(&token).unwrap())
                            .await
                        {
                            error!(error = %e, "Failed to save resume token");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Change stream error");
                }
            }
        }
        Ok(())
    }
}
