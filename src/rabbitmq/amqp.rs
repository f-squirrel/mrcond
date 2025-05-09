use super::error::Error;
use super::publish::Publish;
use crate::config::RabbitMq;
use lapin::{
    options::BasicPublishOptions, publisher_confirm::Confirmation, types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use mongodb::{bson::Document, change_stream::event::ChangeStreamEvent};
use serde_json;
use tracing::info;

pub struct Publisher {
    pub config: RabbitMq,
    channel: Channel,
}

impl Publisher {
    pub async fn new(config: RabbitMq, rabbitmq_uri: &str) -> Result<Self, Error> {
        let conn = Connection::connect(rabbitmq_uri, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;
        channel
            .queue_declare(
                &config.stream_name,
                Default::default(),
                FieldTable::default(),
            )
            .await?;
        Ok(Self { config, channel })
    }

    pub async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error> {
        let payload = serde_json::to_vec(event)?;
        let _confirm: Confirmation = self
            .channel
            .basic_publish(
                "",
                &self.config.stream_name,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?
            .await?;
        info!(queue = %self.config.stream_name, "Published message to RabbitMQ");
        Ok(())
    }
}

#[async_trait::async_trait]
impl Publish for Publisher {
    async fn publish(
        &self,
        event: &mongodb::change_stream::event::ChangeStreamEvent<mongodb::bson::Document>,
    ) -> Result<(), crate::rabbitmq::Error> {
        self.publish(event).await
    }
}
