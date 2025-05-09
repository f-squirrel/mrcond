use crate::config::RabbitMq;

pub struct Publisher {
    pub config: RabbitMq,
}

impl Publisher {
    pub fn new(config: RabbitMq) -> Self {
        Self { config }
    }
    // Add methods for publishing to RabbitMQ here
}
