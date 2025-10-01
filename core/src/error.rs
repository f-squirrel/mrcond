use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("RabbitMQ error: {0}")]
    RabbitMq(#[from] lapin::Error),
    #[error("MongoDB error: {0}")]
    MongoDB(#[from] mongodb::error::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
