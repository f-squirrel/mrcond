pub mod amqp;
pub mod error;
pub mod publish;
pub use error::Error;
use mongodb::{bson::Document, change_stream::event::ChangeStreamEvent};

use crate::rabbitmq::publish::Publish;
use std::sync::Arc;

pub struct Publisher {
    inner: Arc<dyn Publish>,
}

impl Publisher {
    pub fn new(inner: Arc<dyn Publish>) -> Self {
        Self { inner }
    }

    pub async fn publish(&self, event: &ChangeStreamEvent<Document>) -> Result<(), Error> {
        self.inner.publish(event).await
    }
}
