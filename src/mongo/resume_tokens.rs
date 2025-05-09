use mongodb::{
    bson::{doc, Bson, Document},
    Client, Collection, Database,
};
use tracing::error;

pub struct ResumeTokens {
    pub client: Client,
    pub tokens_db_name: String,
    pub tokens_coll_name: String,
    pub tokens_coll_capped: Option<bool>,
    pub tokens_coll_size_in_bytes: Option<u64>,
    db: Database,
    collection: Collection<Document>,
}

impl ResumeTokens {
    pub async fn new(
        client: Client,
        tokens_db_name: String,
        tokens_coll_name: String,
        tokens_coll_capped: Option<bool>,
        tokens_coll_size_in_bytes: Option<u64>,
    ) -> mongodb::error::Result<Self> {
        let db = client.database(&tokens_db_name);
        let collection = db.collection::<Document>(&tokens_coll_name);
        // Optionally create capped collection if requested
        if let Some(true) = tokens_coll_capped {
            let size = tokens_coll_size_in_bytes.unwrap_or(4096) as i64;
            let options_doc = doc! {"capped": true, "size": size};
            let mut create_cmd = doc! {"create": &tokens_coll_name};
            create_cmd.extend(options_doc);
            // Try to create the collection, ignore error if it already exists
            if let Err(e) = db.run_command(create_cmd).await {
                if !e.to_string().contains("already exists") {
                    error!(error = %e, "Failed to create capped collection");
                    return Err(e);
                }
            }
        }
        Ok(Self {
            client,
            tokens_db_name,
            tokens_coll_name,
            tokens_coll_capped,
            tokens_coll_size_in_bytes,
            db,
            collection,
        })
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
        resume_token: Bson,
    ) -> mongodb::error::Result<()> {
        let filter = doc! {"stream_name": stream_name};
        let update = doc! {"$set": {"resume_token": resume_token}};
        self.collection.update_one(filter, update).await?;
        Ok(())
    }
}
