pub mod amqp;
pub mod publish;
pub use amqp::Publisher;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Lapin error: {0}")]
    Lapin(#[from] lapin::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
