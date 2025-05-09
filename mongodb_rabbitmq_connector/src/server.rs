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

    async fn serve(&self) -> Result<(), Error> {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::unbounded_channel();
        let collections = self.settings.collections.clone();
        let settings = self.settings.clone();

        // Helper closure to spawn a resilient job for a collection
        let spawn_job =
            |settings: Settings,
             collection: crate::config::Collection,
             tx: mpsc::UnboundedSender<crate::config::Collection>| {
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
                                if let Err(e) =
                                    connector.connect(&collection.watched.coll_name).await
                                {
                                    error!(error = ?e, collection = %coll_name, "Watcher failed, notifying parent");
                                }
                                let _ = tx.send(collection.clone());
                                break;
                            }
                            Err(e) => {
                                // DD: If one of the components like RabbitMQ or MongoDB is down, we should retry
                                // after a delay.
                                error!(error = ?e, collection = %coll_name, "Failed to create connector, retrying in 5s");
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            }
                        }
                    }
                })
            };

        // Spawn all jobs
        for collection in collections {
            let _handle = spawn_job(settings.clone(), collection.clone(), tx.clone());
        }

        info!("Connector server started");

        while let Some(collection) = rx.recv().await {
            let _h = spawn_job(self.settings.clone(), collection, tx.clone());
            // handles.insert(coll_name, handle);
        }
        Ok(())
    }
}
