use crate::config;
use mongodb::{
    bson::{doc, Bson, Document},
    change_stream::event::ResumeToken,
    Client, Collection,
};
use tracing::error;

pub struct ResumeTokensDB {
    collection: Collection<Document>,
}

impl ResumeTokensDB {
    pub async fn new(
        client: Client,
        config: config::ResumeTokensDB,
    ) -> mongodb::error::Result<Self> {
        let db = client.database(&config.tokens_db_name);
        let collection = db.collection::<Document>(&config.tokens_coll_name);
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

    pub async fn get_last_resume_token(
        &self,
        stream_name: &str,
    ) -> mongodb::error::Result<Option<Bson>> {
        let filter = doc! {"stream_name": stream_name};
        match self.collection.find_one(filter).await? {
            Some(doc) => Ok(doc.get("resume_token").cloned()),
            None => Ok(None),
        }
    }

    pub async fn set_last_resume_token(
        &self,
        stream_name: &str,
        resume_token: &ResumeToken,
    ) -> mongodb::error::Result<()> {
        let filter = doc! {"stream_name": stream_name};
        let update = doc! {"$set": {"resume_token": bson::to_bson(resume_token)?}};

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
