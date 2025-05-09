//! Server module for running the connector as a service

use crate::config::Settings;
use thiserror::Error;
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

    fn spawn(
        settings: Settings,
        collection: crate::config::Collection,
        tx: tokio::sync::mpsc::UnboundedSender<crate::config::Collection>,
    ) -> tokio::task::JoinHandle<()> {
        let coll_name = collection.watched.coll_name.clone();
        tokio::spawn(async move {
            loop {
                match crate::mongo::connector::Connector::from_collection(
                    &settings.mongo_uri,
                    &settings.rabbitmq_uri,
                    &collection,
                )
                .await
                {
                    Ok(connector) => {
                        if let Err(e) = connector.connect(&collection.watched.coll_name).await {
                            tracing::error!(error = ?e, collection = %coll_name, "Watcher failed, notifying parent");
                        }
                        let _ = tx.send(collection.clone());
                        break;
                    }
                    Err(e) => {
                        tracing::error!(error = ?e, collection = %coll_name, "Failed to create connector, retrying in 5s");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        })
    }

    pub async fn serve(&self) -> Result<(), Error> {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::unbounded_channel();
        let collections = self.settings.collections.clone();
        let settings = self.settings.clone();

        // Spawn all jobs
        for collection in collections {
            info!(collection = %collection.watched.coll_name, "Starting connector for collection");
            Self::spawn(settings.clone(), collection, tx.clone());
        }

        info!("Connector server started");

        while let Some(collection) = rx.recv().await {
            info!(collection = %collection.watched.coll_name, "Restarting connector for collection");
            Self::spawn(self.settings.clone(), collection, tx.clone());
        }
        Ok(())
    }
}
