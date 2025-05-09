//! Server module for running the connector as a service

use crate::config::Settings;
use crate::mongo::resume_tokens::ResumeTokensDB;
use crate::mongo::connector::Connector;
use crate::rabbitmq::{amqp, Publisher};
use lapin::protocol::connection;
use mongodb::Client;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::watch;
use tracing::{error, info};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Watcher error: {0}")]
    Watcher(#[from] crate::mongo::connector::Error),
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
        for collection in &self.settings.collections {
            let settings = self.settings.clone();
            let collection = collection.clone();

            let _handle = tokio::spawn(async move {
                let connector = crate::mongo::connector::Connector::from_collection(
                    &settings.mongo_uri,
                    &settings.rabbitmq_uri,
                    &collection,
                )
                .await
                .unwrap();

                if let Err(e) = connector.watch(&collection.watched.coll_name).await {
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
