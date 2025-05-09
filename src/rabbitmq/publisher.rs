use crate::config::RabbitMq;
use lapin::{
    options::BasicPublishOptions, publisher_confirm::Confirmation, types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use mongodb::{bson::Document, change_stream::event::ChangeStreamEvent};
use serde_json;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Lapin error: {0}")]
    Lapin(#[from] lapin::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

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
