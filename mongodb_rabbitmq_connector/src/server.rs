//! Server module for running the connector as a service

use std::{sync::Arc, time::Duration};

use crate::{config::Settings, rabbitmq};
use mongodb::Client;
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

    async fn connect_to_mongo(settings: &Settings) -> Result<Client, Error> {
        loop {
            match Client::with_uri_str(settings.connections().mongo_uri.as_str()).await {
                Ok(client) => {
                    info!("MongoDB connection initialized successfully");
                    return Ok(client);
                }
                Err(e) => {
                    error!(error = ?e, "Failed to initialize MongoDB connection");
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    async fn connect_to_rabbitmq(settings: &Settings) -> Result<Arc<lapin::Connection>, Error> {
        loop {
            match lapin::Connection::connect(
                settings.connections().rabbitmq_uri.as_str(),
                lapin::ConnectionProperties::default(),
            )
            .await
            {
                Ok(client) => {
                    info!("RabbitMQ connection initialized successfully");
                    return Ok(Arc::new(client));
                }
                Err(e) => {
                    error!(error = ?e, "Failed to initialize RabbitMQ connection");
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    /// Connect to MongoDB and RabbitMQ, returning both clients.
    ///
    /// This function is independent of `self` and takes a `Settings` reference.
    async fn connect_clients(
        settings: &Settings,
    ) -> Result<(mongodb::Client, Arc<lapin::Connection>), Error> {
        let (mongo_client, rabbitmq_client) = tokio::try_join!(
            Self::connect_to_mongo(settings),
            Self::connect_to_rabbitmq(settings)
        )?;
        Ok((mongo_client, rabbitmq_client))
    }

    async fn spawn_task(
        collection: crate::config::Collection,
        mongo_client: mongodb::Client,
        rabbitmq_client: Arc<lapin::Connection>,
    ) -> Result<(), Error> {
        let coll_name = collection.watched.coll_name.clone();
        loop {
            match crate::mongo::connector::Connector::with_clients(
                mongo_client.clone(),
                rabbitmq_client.clone(),
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

        let (mut mongo_client, mut rabbitmq_client) = Self::connect_clients(&self.settings).await?;

        let collections = self.settings.collections();
        let settings = self.settings.clone();
        let mut join_set = JoinSet::new();

        for collection in collections {
            info!(collection = %collection.watched.coll_name, "Starting connector for collection");
            join_set.spawn(Self::spawn_task(
                collection.clone(),
                mongo_client.clone(),
                rabbitmq_client.clone(),
            ));
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
                        Error::Connector { source, collection } => {
                            error!(error = ?source, collection = %collection.watched.coll_name, "Connector task failed, restarting");
                            match source {
                                crate::mongo::connector::Error::Mongo(_) => {
                                    info!("Restarting mongo client");
                                    mongo_client = Self::connect_to_mongo(&settings).await?;
                                }
                                crate::mongo::connector::Error::RabbitMq(
                                    rabbitmq::Error::Lapin(_),
                                ) => {
                                    info!("Restarting RabbitMQ client");
                                    rabbitmq_client = Self::connect_to_rabbitmq(&settings).await?;
                                }
                                other => {
                                    error!(error = ?other, "Unhandled connector error, not restarting");
                                }
                            }
                            join_set.spawn(Server::spawn_task(
                                collection,
                                mongo_client.clone(),
                                rabbitmq_client.clone(),
                            ));
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
