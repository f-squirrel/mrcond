//! Server module for running the connector as a service

use std::{sync::Arc, time::Duration};

use crate::{config::Settings, metrics::Metrics, rabbitmq};
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
/// - `metrics`: Prometheus metrics collector for tracking server status.
pub struct Server {
    settings: Settings,
    metrics: Metrics,
}

const RETRY_DELAY: Duration = Duration::from_secs(5);

impl Server {
    /// Create a new `Server` with the given settings.
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            metrics: Metrics::new(),
        }
    }

    /// Create a new `Server` with the given settings and metrics collector.
    pub fn with_metrics(settings: Settings, metrics: Metrics) -> Self {
        Self { settings, metrics }
    }

    /// Get a reference to the metrics collector.
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
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
        let connector = crate::mongo::connector::Connector::with_clients(
            mongo_client.clone(),
            rabbitmq_client.clone(),
            &collection,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, collection = %coll_name, "Failed to create connector");
            Error::Connector {
                source: e,
                collection: collection.clone(),
            }
        })?;

        connector.connect(&collection.watched.coll_name).await.map_err(|e| {
            tracing::error!(error = ?e, collection = %coll_name, "Failed to connect to collection");
            Error::Connector {
                source: e,
                collection: collection.clone(),
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
        use tokio::task::JoinSet;

        let (mut mongo_client, mut rabbitmq_client) = Self::connect_clients(&self.settings).await?;

        let collections = self.settings.collections();
        let mut join_set = JoinSet::new();

        for collection in collections {
            info!(collection = %collection.watched.coll_name, "Starting connector for collection");
            join_set.spawn(Self::spawn_task(
                collection.clone(),
                mongo_client.clone(),
                rabbitmq_client.clone(),
            ));
            self.metrics.increment_collection_server(
                &collection.watched.coll_name,
                &collection.watched.db_name,
            );
            self.metrics.record_task_start();
        }

        // Update total server count
        self.metrics.set_server_count(join_set.len());

        info!("Connector server started");
        while let Some(res) = join_set.join_next().await {
            // Update metrics after a task completes
            self.metrics.set_server_count(join_set.len());

            match res {
                Ok(Ok(_)) => {
                    warn!("Connector task finished due to collection drop, not restarting");
                }
                Ok(Err(e)) => match e {
                    Error::Connector { source, collection } => {
                        error!(error = ?source, collection = %collection.watched.coll_name, "Connector task failed, restarting");

                        // Record the failure and restart reason
                        let (error_type, restart_reason) = match &source {
                            crate::mongo::connector::Error::Mongo(_) => {
                                info!("Restarting mongo client");
                                mongo_client = Self::connect_to_mongo(&self.settings).await?;
                                ("mongo_error", "mongo_connection_failed")
                            }
                            crate::mongo::connector::Error::RabbitMq(rabbitmq::Error::Lapin(_)) => {
                                info!("Restarting RabbitMQ client");
                                rabbitmq_client = Self::connect_to_rabbitmq(&self.settings).await?;
                                ("rabbitmq_error", "rabbitmq_connection_failed")
                            }
                            other => {
                                error!(error = ?other, "Unhandled connector error, reusing existing clients");
                                ("unknown_error", "unhandled_error")
                            }
                        };

                        self.metrics.record_task_failure(
                            &collection.watched.coll_name,
                            &collection.watched.db_name,
                            error_type,
                        );
                        self.metrics.record_task_restart(
                            &collection.watched.coll_name,
                            &collection.watched.db_name,
                            restart_reason,
                        );

                        join_set.spawn(Server::spawn_task(
                            collection.clone(),
                            mongo_client.clone(),
                            rabbitmq_client.clone(),
                        ));
                        self.metrics.record_task_start();
                        // Update metrics when restarting a task
                        self.metrics.set_server_count(join_set.len());
                    }
                },
                Err(e) => {
                    error!(error = ?e, "Connector task panicked, not restarting");
                    // Record the panic as a failure (we can't determine which collection, so use "unknown")
                    self.metrics
                        .record_task_failure("unknown", "unknown", "task_panic");
                    // Update metrics when a task panics and is not restarted
                    self.metrics.set_server_count(join_set.len());
                }
            }
        }

        info!("Connector server tasks are finished");

        Ok(())
    }
}
