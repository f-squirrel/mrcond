use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use bson::Document;
use lapin::{options::*, types::FieldTable, ConnectionProperties};
use mongodb::Database;
use mongodb::{bson::doc, Client};
use mongodb_rabbitmq_connector::config::{Collection, Settings};
use serde::Deserialize;

/// Producer for inserting documents into MongoDB.
pub struct Producer {
    client: mongodb::Client,
    db: Database,
    collection: mongodb::Collection<Document>,
}

impl Producer {
    /// Initialize a Producer from a config file and a mongo_uri override.
    pub async fn mew(client: mongodb::Client, collection: &Collection) -> Self {
        let db = client.database(&collection.watched.db_name);
        let collection = db.collection::<Document>(collection.watched.coll_name.as_str());
        Self {
            client,
            db,
            collection,
        }
    }

    pub async fn send_bulk(&self, docs: &[serde_json::Value]) -> mongodb::error::Result<()> {
        self.collection.delete_many(doc! {}).await?;
        if !docs.is_empty() {
            let docs: Vec<_> = docs
                .iter()
                .map(|v| mongodb::bson::to_document(v).unwrap())
                .collect();
            self.collection.insert_many(docs).await?;
        }
        Ok(())
    }
}

/// Consumer for receiving messages from RabbitMQ.
pub struct Consumer {
    channel: lapin::Channel,
    queue: String,
}

impl Consumer {
    pub async fn new(rabbitmq_uri: &str, queue: &str) -> lapin::Result<Self> {
        let conn =
            lapin::Connection::connect(rabbitmq_uri, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;
        channel
            .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
            .await?;
        Ok(Self {
            channel,
            queue: queue.to_string(),
        })
    }

    pub async fn receive_all(&self, expected: usize, max_tries: usize) -> Vec<serde_json::Value> {
        let mut received = vec![];
        let mut tries = 0;
        while received.len() < expected && tries < max_tries {
            let msg = self
                .channel
                .basic_get(&self.queue, BasicGetOptions::default())
                .await
                .unwrap();
            if let Some(data) = msg {
                let v: serde_json::Value = serde_json::from_slice(&data.data).unwrap();
                received.push(v);
            } else {
                sleep(Duration::from_millis(200));
                tries += 1;
            }
        }
        received
    }
}
