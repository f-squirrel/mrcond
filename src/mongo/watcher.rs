use futures_util::stream::StreamExt;
use mongodb::{bson::Document, Client};
use tracing::{debug, error, info};

pub struct Watcher {
    pub client: Client,
    pub db_name: String,
    pub coll_name: String,
    pub change_stream_pre_and_post_images: bool,
}

impl Watcher {
    pub fn new(
        client: Client,
        db_name: String,
        coll_name: String,
        change_stream_pre_and_post_images: bool,
    ) -> Self {
        Self {
            client,
            db_name,
            coll_name,
            change_stream_pre_and_post_images,
        }
    }

    pub async fn watch(&self) -> mongodb::error::Result<()> {
        let collection = self
            .client
            .database(&self.db_name)
            .collection::<Document>(&self.coll_name);
        let mut change_stream = collection.watch().await?;
        info!(db = %self.db_name, coll = %self.coll_name, "Started watching collection");
        while let Some(event) = change_stream.next().await {
            match event {
                Ok(change) => {
                    debug!(?change, "Change event");
                }
                Err(e) => {
                    error!(error = %e, "Change stream error");
                }
            }
        }
        Ok(())
    }
}
