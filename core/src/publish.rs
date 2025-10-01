use async_trait::async_trait;
use mongodb::{bson::Document, change_stream::event::ChangeStreamEvent};

use crate::Error;

#[async_trait]
pub trait Publish: Send + Sync + 'static {
    async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error>;
}
