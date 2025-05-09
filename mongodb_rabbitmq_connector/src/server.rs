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

    async fn do_run(&self) -> Result<(), Error> {
        use std::collections::HashMap;
        use tokio::task::JoinHandle;

        let mut handles: HashMap<String, JoinHandle<()>> = HashMap::new();
        for collection in &self.settings.collections {
            let settings = self.settings.clone();
            let collection = collection.clone();
            let coll_name = collection.watched.coll_name.clone();

            let spawn_job = move || {
                tokio::spawn(async move {
                    loop {
                        let connector = match crate::mongo::connector::Connector::from_collection(
                            &settings.mongo_uri,
                            &settings.rabbitmq_uri,
                            &collection,
                        )
                        .await
                        {
                            Ok(conn) => conn,
                            Err(e) => {
                                error!(error = ?e, "Failed to create connector, retrying in 5s");
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                continue;
                            }
                        };
                        if let Err(e) = connector.watch(&collection.watched.coll_name).await {
                            error!(error = ?e, "Watcher failed, restarting in 5s");
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        } else {
                            // If watch returns Ok, break loop (should not happen in normal operation)
                            break;
                        }
                    }
                })
            };

            let handle = spawn_job();
            handles.insert(coll_name.clone(), handle);
        }

        info!("Connector server started");
        // Optionally: monitor jobs and respawn if they exit unexpectedly
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let mut to_respawn = vec![];
            for (coll_name, handle) in handles.iter_mut() {
                if handle.is_finished() {
                    error!(collection = %coll_name, "Job exited unexpectedly, respawning");
                    to_respawn.push(coll_name.clone());
                }
            }
            for coll_name in to_respawn {
                let settings = self.settings.clone();
                let collection = self
                    .settings
                    .collections
                    .iter()
                    .find(|c| c.watched.coll_name == coll_name)
                    .unwrap()
                    .clone();
                let spawn_job = move || {
                    tokio::spawn(async move {
                        loop {
                            let connector =
                                match crate::mongo::connector::Connector::from_collection(
                                    &settings.mongo_uri,
                                    &settings.rabbitmq_uri,
                                    &collection,
                                )
                                .await
                                {
                                    Ok(conn) => conn,
                                    Err(e) => {
                                        error!(error = ?e, "Failed to create connector, retrying in 5s");
                                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                        continue;
                                    }
                                };
                            if let Err(e) = connector.watch(&collection.watched.coll_name).await {
                                error!(error = ?e, "Watcher failed, restarting in 5s");
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            } else {
                                break;
                            }
                        }
                    })
                };
                let handle = spawn_job();
                handles.insert(coll_name.clone(), handle);
            }
        }
        // Ok(())
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.do_run().await.map_err(|e| {
            error!(error = ?e, "Connector server failed");
            Box::new(e) as Box<dyn std::error::Error>
        })
    }
}
