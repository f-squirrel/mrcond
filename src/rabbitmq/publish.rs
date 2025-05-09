use async_trait::async_trait;
use mongodb::{bson::Document, change_stream::event::ChangeStreamEvent};

#[async_trait]
pub trait Publish {
    async fn publish(
        &self,
        event: &ChangeStreamEvent<Document>,
    ) -> Result<(), crate::rabbitmq::Error>;
}
