//! Server module for running the connector as a service

use std::time::Duration;

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

/// Supervisor for running and restarting MongoDB-to-RabbitMQ connector jobs.
///
/// The `Server` manages the lifecycle of connector tasks for each configured collection. It supervises
/// connector jobs, restarts them on failure, and provides a main entrypoint for running the connector as a service.
///
/// Fields:
/// - `settings`: Application settings, including all collection and connection configuration.
pub struct Server {
    settings: Settings,
}

const RETRY_DELAY: Duration = Duration::from_secs(5);

impl Server {
    /// Create a new `Server` with the given settings.
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Spawn a connector job for a specific collection.
    ///
    /// This function supervises the connector: if the connector fails to start or run, it will retry after a delay.
    /// On error, the collection is sent back to the parent for restart.
    fn spawn(
        settings: Settings,
        collection: crate::config::Collection,
        tx: tokio::sync::mpsc::UnboundedSender<crate::config::Collection>,
    ) -> tokio::task::JoinHandle<()> {
        let coll_name = collection.watched.coll_name.clone();
        tokio::spawn(async move {
            loop {
                match crate::mongo::connector::Connector::from_collection(
                    &settings.connections.mongo_uri,
                    &settings.connections.rabbitmq_uri,
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
                        tracing::error!(error = ?e, collection = %coll_name, "Failed to create connector, retrying in {RETRY_DELAY:?}");
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                }
            }
        })
    }

    /// Run the connector server, supervising all collection jobs.
    ///
    /// This method spawns a connector job for each configured collection and supervises their lifecycle.
    /// If a job fails, it is restarted. The server runs until all jobs have exited or the process is terminated.
    ///
    /// # Errors
    /// Returns an error if a fatal error occurs in the supervision loop.
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
