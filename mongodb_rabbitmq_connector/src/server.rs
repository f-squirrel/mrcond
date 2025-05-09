//! Server module for running the connector as a service

use crate::config::Settings;
use crate::mongo::resume_tokens::ResumeTokensDB;
use crate::mongo::watcher::Watcher;
use crate::rabbitmq::{amqp, Publisher};
use futures_util::stream;
use mongodb::Client;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Watcher error: {0}")]
    Watcher(#[from] crate::mongo::watcher::Error),
    #[error("RabbitMQ error: {0}")]
    RabbitMq(#[from] crate::rabbitmq::Error),
    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),
}

pub struct ConnectorServer {
    settings: Settings,
}

impl ConnectorServer {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    async fn do_run(&self) -> Result<(), Error> {
        let client = Client::with_uri_str(&self.settings.mongo_uri).await?;
        for collection in &self.settings.collections {
            let resume_tokens =
                ResumeTokensDB::new(client.clone(), collection.resume_tokens.clone()).await?;
            let amqp_publisher =
                amqp::Publisher::new(&collection.rabbitmq, &self.settings.rabbitmq_uri).await?;
            let publisher = Publisher::new(Arc::new(amqp_publisher));
            let watcher = Watcher::new(
                client.clone(),
                collection.watched.clone(),
                resume_tokens,
                publisher,
            )
            .await?;
            let stream_name = collection.rabbitmq.stream_name.clone();
            // In production, spawn each watcher as a task
            let handle = tokio::spawn(async move {
                if let Err(e) = watcher.watch(&stream_name).await {
                    error!(error = ?e, "Watcher failed");
                }
            });
        }
        info!("Connector server started");
        // Keep running
        futures_util::future::pending::<()>().await;
        Ok(())
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.do_run().await.map_err(|e| {
            error!(error = ?e, "Connector server failed");
            Box::new(e) as Box<dyn std::error::Error>
        })
    }
}
