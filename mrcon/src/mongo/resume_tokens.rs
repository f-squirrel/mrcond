use crate::config;
use mongodb::{Client, Collection, bson::doc, change_stream::event::ResumeToken};
use serde::{Deserialize, Serialize};
use tracing::error;

/// Persistent storage for MongoDB change stream resume tokens.
///
/// `ResumeTokensDB` manages the storage and retrieval of resume tokens for MongoDB change streams,
/// allowing the connector to reliably resume streaming after restarts or failures. Tokens are stored
/// in a dedicated MongoDB collection, optionally capped for bounded storage.
pub struct ResumeTokensDB {
    collection: Collection<ResumeTokenDbView>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResumeTokenDbView {
    stream_name: String,
    resume_token: ResumeToken,
}

impl ResumeTokensDB {
    /// Create a new `ResumeTokensDB` instance with the given MongoDB client and configuration.
    ///
    /// Optionally creates a capped collection for token storage if requested in the config.
    ///
    /// # Arguments
    /// * `client` - MongoDB client instance.
    /// * `config` - Configuration for the resume token database and collection.
    ///
    /// # Errors
    /// Returns an error if the collection cannot be created or accessed.
    pub async fn new(
        client: Client,
        config: config::ResumeTokensDB,
    ) -> mongodb::error::Result<Self> {
        let db = client.database(&config.tokens_db_name);
        let collection = db.collection::<ResumeTokenDbView>(&config.tokens_coll_name);
        // Optionally create capped collection if requested
        if let Some(true) = config.tokens_coll_capped {
            let size = config.tokens_coll_size_in_bytes.unwrap_or(4096) as i64;
            let options_doc = doc! {"capped": true, "size": size};
            let mut create_cmd = doc! {"create": &config.tokens_coll_name};
            create_cmd.extend(options_doc);
            // Try to create the collection, ignore error if it already exists
            if let Err(e) = db.run_command(create_cmd).await {
                if !e.to_string().contains("already exists") {
                    error!(error = %e, "Failed to create capped collection");
                    return Err(e);
                }
            }
        }
        Ok(Self { collection })
    }

    /// Remove all stored resume tokens from the collection.
    ///
    /// # Errors
    /// Returns an error if the delete operation fails.
    pub async fn clean(&self) -> mongodb::error::Result<()> {
        self.collection.delete_many(doc! {}).await?;
        Ok(())
    }

    /// Retrieve the last stored resume token for a given stream.
    ///
    /// # Arguments
    /// * `stream_name` - The unique identifier for the change stream.
    ///
    /// # Returns
    /// The last stored resume token as `Option<ResumeToken>`
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn get_last_resume_token(
        &self,
        stream_name: &str,
    ) -> mongodb::error::Result<Option<ResumeToken>> {
        let filter = doc! {"stream_name": stream_name};
        let doc = self.collection.find_one(filter).await?;
        Ok(doc.map(|d| d.resume_token))
    }

    /// Store or update the last resume token for a given stream (upsert semantics).
    ///
    /// # Arguments
    /// * `stream_name` - The unique identifier for the change stream.
    /// * `resume_token` - The resume token to persist.
    ///
    /// # Errors
    /// Returns an error if the update operation fails.
    pub async fn set_last_resume_token(
        &self,
        stream_name: &str,
        resume_token: &ResumeToken,
    ) -> mongodb::error::Result<()> {
        let filter = doc! {"stream_name": stream_name};
        let update = doc! {"$set": {"resume_token": mongodb::bson::to_bson(resume_token)?}};

        // Use upsert: true to insert if not exists, or update if exists
        let options = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();
        self.collection
            .update_one(filter, update)
            .with_options(Some(options))
            .await?;
        Ok(())
    }
}
