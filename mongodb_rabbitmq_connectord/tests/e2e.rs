use futures_util::stream::StreamExt;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use bson::Document;
use lapin::{options::*, types::FieldTable, ConnectionProperties};
use mongodb::Client;
use mongodb::Database;
use mongodb_rabbitmq_connector::config::{Collection, Connections, Settings};

struct Cluster {
    cluster: Child,
    running: bool,
}

impl Cluster {
    fn start() -> Self {
        let cluster = Command::new("make")
            .args(&["-C", "../", "down", "run"])
            .stdout(Stdio::null())
            .spawn()
            .expect("Failed to start cluster");
        Self {
            cluster,
            running: true,
        }
    }

    fn stop(&mut self) {
        if !self.running {
            return;
        }

        let _ = self.cluster.kill();
        let _ = self.cluster.wait();

        let _ = Command::new("make")
            .args(&["-C", "../", "down"])
            .stdout(Stdio::null())
            .spawn()
            .expect("Failed to start cluster")
            .wait()
            .unwrap();

        self.running = false;
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Producer for inserting documents into MongoDB.
#[derive(Clone)]
pub struct Producer {
    _client: mongodb::Client,
    _db: Database,
    collection: mongodb::Collection<Document>,
}

impl Producer {
    /// Initialize a Producer from a config file and a mongo_uri override.
    pub async fn new(client: mongodb::Client, collection: &Collection) -> Self {
        println!("Connecting to MongoDB at {}", collection.watched.db_name);
        let db = client.database(&collection.watched.db_name);
        println!(
            "Using collection {} in database {}",
            collection.watched.coll_name, collection.watched.db_name
        );
        let collection = db.collection::<Document>(collection.watched.coll_name.as_str());
        Self {
            _client: client,
            _db: db,
            collection,
        }
    }

    pub async fn send_bulk(&self, docs: &[serde_json::Value]) -> mongodb::error::Result<()> {
        if !docs.is_empty() {
            let docs: Vec<_> = docs
                .iter()
                .map(|v| mongodb::bson::to_document(v).unwrap())
                .collect();
            self.collection.insert_many(docs).await?;
            println!("Inserted documents into MongoDB");
        } else {
            println!("No documents to insert");
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
        let mut counter = 0;

        let conn = loop {
            match lapin::Connection::connect(rabbitmq_uri, ConnectionProperties::default()).await {
                Ok(conn) => {
                    println!("Connected to RabbitMQ at {}", rabbitmq_uri);
                    break conn;
                }
                Err(e) => {
                    println!("Failed to connect to RabbitMQ: {}. Retrying...", e);
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    counter += 1;
                    if counter >= 10 {
                        panic!("Failed to connect to RabbitMQ after 10 attempts");
                    }
                }
            }
        };

        // let conn =
        //     lapin::Connection::connect(rabbitmq_uri, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;
        channel
            .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
            .await?;
        Ok(Self {
            channel,
            queue: queue.to_string(),
        })
    }

    pub async fn receive_all(&self, expected: usize) -> Vec<serde_json::Value> {
        let mut received = vec![];
        let mut consumer = self
            .channel
            .basic_consume(
                &self.queue,
                "my tag",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        while let Some(data) = consumer.next().await {
            let data = data.unwrap();
            let v: serde_json::Value = serde_json::from_slice(&data.data).unwrap();
            received.push(v);
            if received.len() >= expected {
                break;
            }
        }
        received
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_producer_consumer_init_and_join() {
    let base = std::env::current_dir().unwrap();
    println!("Current directory: {:?}", base);

    let mut cluster = Cluster::start();
    tokio::time::sleep(Duration::from_secs(10)).await;

    let config = config::Config::builder()
        .add_source(config::File::with_name("tests/data/simple/config.yaml"))
        .build()
        .unwrap();

    let settings = config.try_deserialize::<Settings>().unwrap();

    let config = config::Config::builder()
        .add_source(config::File::with_name(
            "tests/data/simple/connections.yaml",
        ))
        .build()
        .unwrap();
    let connections = config.try_deserialize::<Connections>().unwrap();

    let settings = Settings::new(connections, settings.collections().to_owned())
        .map_err(|e| anyhow::anyhow!("Failed to create settings: {}", e))
        .unwrap();

    println!("Settings: {:?}", settings);

    let mut counter = 0;
    let client = loop {
        match Client::with_uri_str(settings.connections().mongo_uri.clone()).await {
            Ok(client) => {
                println!(
                    "Connected to MongoDB at {}",
                    settings.connections().mongo_uri
                );
                break client;
            }
            Err(e) => {
                if counter >= 10 {
                    panic!("Failed to connect to MongoDB after 10 attempts: {}", e);
                }
                println!("Failed to connect to MongoDB: {}. Retrying...", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                counter += 1;
            }
        }
    };
    // let client = Client::with_uri_str(settings.connections.mongo_uri)
    //     .await
    //     .unwrap();

    let producer = Producer::new(client, &settings.collections()[0]).await;
    let consumer = Consumer::new(
        &settings.connections().rabbitmq_uri,
        &settings.collections()[0].rabbitmq.stream_name,
    )
    .await
    .unwrap();

    // Read input documents from file
    let docs: Vec<serde_json::Value> = {
        let cwd = std::env::current_dir().unwrap();
        println!("Current directory: {:?}", cwd);
        let file = std::fs::File::open("tests/data/simple/input.json").unwrap();
        serde_json::from_reader(file).unwrap()
    };
    let expected = docs.len();

    println!("Sending {} documents to MongoDB", docs.len());
    let res = producer.send_bulk(&docs).await;
    println!("Producer finished sending documents: {:?}", res);
    // Spawn producer and consumer tasks
    // let producer_task = tokio::spawn({
    //     let producer = producer.clone();
    //     let docs = docs.clone();
    //     async move {
    //         println!("Sending {} documents to MongoDB", docs.len());
    //         let res = producer.send_bulk(&docs).await;
    //         println!("Producer finished sending documents: {:?}", res);
    //     }
    // });

    // tokio::time::sleep(Duration::from_secs(100)).await; // Wait for producer to send documents
    let output = consumer.receive_all(expected).await;
    println!("Consumer received {} documents", output.len());

    output.iter().enumerate().for_each(|(i, doc)| {
        println!("Document {}: {:?}", i, doc);
        let full = doc.get("fullDocument").unwrap();
        println!("Document {}: {:?}", i, full);
    });

    let mut full_docs = Vec::new();
    output.iter().enumerate().for_each(|(i, doc)| {
        println!("Document {}: {:?}", i, doc);
        if let Some(mut full) = doc.get("fullDocument").cloned() {
            // Remove the "_id" field if present
            if let Some(obj) = full.as_object_mut() {
                obj.remove("_id");
            }
            println!("Document {} (fullDocument, no _id): {:?}", i, full);
            full_docs.push(full);
        }
    });
    assert_eq!(docs, full_docs);

    // let consumer_task = tokio::spawn(async move {
    //     let received = consumer.receive_all(expected, 1000).await;
    //     assert_eq!(received, docs);
    // });

    // // Join both tasks
    // let _ = tokio::try_join!(producer_task, consumer_task).unwrap();
    cluster.stop();
}
