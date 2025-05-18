use config;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use bson::Document;
use lapin::{options::*, types::FieldTable, ConnectionProperties};
use mongodb::Database;
use mongodb::{bson::doc, Client};
use mongodb_rabbitmq_connector::config::{Collection, Connections, Settings};
use serde::Deserialize;

fn start_cluster() -> Child {
    let cluster = Command::new("make")
        .args(&["down", "run"])
        .stdout(Stdio::null())
        .spawn()
        .expect("Failed to start cluster");

    cluster
}

fn stop_cluster(cluster: &mut Child) {
    let _ = cluster.kill();
    let _ = cluster.wait();
}

/// Producer for inserting documents into MongoDB.
#[derive(Clone)]
pub struct Producer {
    client: mongodb::Client,
    db: Database,
    collection: mongodb::Collection<Document>,
}

impl Producer {
    /// Initialize a Producer from a config file and a mongo_uri override.
    pub async fn new(client: mongodb::Client, collection: &Collection) -> Self {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_producer_consumer_init_and_join() {
    let config = config::Config::builder()
        .add_source(config::File::with_name("data/simple/config.yaml"))
        .build()
        .unwrap();

    let settings = config.try_deserialize::<Settings>().unwrap();

    let config = config::Config::builder()
        .add_source(config::File::with_name("data/simple/connections.yaml"))
        .build()
        .unwrap();
    let connections = config.try_deserialize::<Connections>().unwrap();

    let settings = Settings {
        connections,
        collections: settings.collections,
    };

    let client = Client::with_uri_str(settings.connections.mongo_uri)
        .await
        .unwrap();

    let producer = Producer::new(client, &settings.collections[0]).await;
    let consumer = Consumer::new(
        &settings.connections.rabbitmq_uri,
        &settings.collections[0].rabbitmq.stream_name,
    )
    .await
    .unwrap();

    // Read input documents from file
    let docs: Vec<serde_json::Value> = {
        let file = fs::File::open("data/simple/input.json").unwrap();
        serde_json::from_reader(file).unwrap()
    };
    let expected = docs.len();

    // Spawn producer and consumer tasks
    let producer_task = tokio::spawn({
        let producer = producer.clone();
        let docs = docs.clone();
        async move {
            producer.send_bulk(&docs).await.unwrap();
        }
    });
    let consumer_task = tokio::spawn(async move {
        let received = consumer.receive_all(expected, 20).await;
        assert_eq!(received, docs);
    });

    // Join both tasks
    let _ = tokio::try_join!(producer_task, consumer_task).unwrap();
}
