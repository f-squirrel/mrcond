use crate::config::RabbitMq;
use lapin::{
    options::BasicPublishOptions, publisher_confirm::Confirmation, types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use serde_json;
use tracing::info;

pub struct Publisher {
    pub config: RabbitMq,
    channel: Channel,
}

impl Publisher {
    pub async fn new(config: RabbitMq, rabbitmq_uri: &str) -> Result<Self, lapin::Error> {
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

    pub async fn publish<T: serde::Serialize>(&self, message: &T) -> Result<(), lapin::Error> {
        let payload = serde_json::to_vec(message)
            .map_err(|e| lapin::Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
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
