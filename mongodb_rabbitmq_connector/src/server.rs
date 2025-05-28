//! Server module for running the connector as a service

use std::time::Duration;

use crate::config::Settings;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Connector error in collection '{collection:?}': {source}")]
    Connector {
        #[source]
        source: crate::mongo::connector::Error,
        collection: crate::config::Collection,
    },
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

    async fn spawn_task(
        settings: Settings,
        collection: crate::config::Collection,
    ) -> Result<(), Error> {
        let coll_name = collection.watched.coll_name.clone();
        loop {
            match crate::mongo::connector::Connector::from_collection(
                &settings.connections().mongo_uri,
                &settings.connections().rabbitmq_uri,
                &collection,
            )
            .await
            {
                Ok(connector) => {
                    let x = connector.connect(&collection.watched.coll_name).await;
                    return x.map_err(|e| {
                        tracing::error!(error = ?e, collection = %coll_name, "Watcher failed, notifying parent");
                        Error::Connector {
                            source: e,
                            collection: collection.clone(),
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = ?e, collection = %coll_name, "Failed to create connector, retrying in {RETRY_DELAY:?}");
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    /// Run the connector server, supervising all collection jobs.
    ///
    /// This method spawns a connector job for each configured collection and supervises their lifecycle.
    /// If a job fails, it is restarted. The server runs until all jobs have exited or the process is terminated.
    ///
    /// # Errors
    /// Returns an error if a fatal error occurs in the supervision loop.
    pub async fn serve(&self) -> Result<(), Error> {
        use tokio::task::JoinSet;

        let collections = self.settings.collections();
        let settings = self.settings.clone();
        let mut join_set = JoinSet::new();

        for collection in collections {
            info!(collection = %collection.watched.coll_name, "Starting connector for collection");
            join_set.spawn(Self::spawn_task(settings.clone(), collection.clone()));
        }

        info!("Connector server started");
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(_)) => {
                    warn!("Connector task finished due to collection drop, not restarting");
                }
                Ok(Err(e)) => {
                    error!(error = ?e, "Connector task failed, restarting");
                    match e {
                        Error::Connector {
                            source: _,
                            collection,
                        } => {
                            join_set.spawn(Server::spawn_task(settings.clone(), collection));
                        }
                    }
                }
                Err(e) => {
                    error!(error = ?e, "Connector task panicked, not restarting");
                }
            }
        }

        info!("Connector server tasks are finished");

        Ok(())
    }
}
