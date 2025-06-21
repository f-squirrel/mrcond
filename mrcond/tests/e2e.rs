use futures_util::stream::StreamExt;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use bson::Document;
use lapin::{ConnectionProperties, options::*, types::FieldTable};
use mongodb::Client;
use mongodb::Database;
use mrcon::config::{Collection, Connections, Settings};

struct Cluster {
    cluster: Child,
    running: bool,
}

impl Cluster {
    fn start() -> Self {
        println!("Starting docker-compose cluster...");

        // First, make sure we're starting clean
        let _ = Command::new("make")
            .args(["-C", "../", "down"])
            .output()
            .expect("Failed to run make down");

        let mut cluster = Command::new("make")
            .args(["-C", "../", "run"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start cluster");

        // Capture and print stdout in a separate thread
        if let Some(stdout) = cluster.stdout.take() {
            let reader = BufReader::new(stdout);
            thread::spawn(move || {
                for line in reader.lines().map_while(Result::ok) {
                    println!("[DOCKER-COMPOSE STDOUT] {}", line);
                }
            });
        }

        // Capture and print stderr in a separate thread
        if let Some(stderr) = cluster.stderr.take() {
            let reader = BufReader::new(stderr);
            thread::spawn(move || {
                for line in reader.lines().map_while(Result::ok) {
                    println!("[DOCKER-COMPOSE STDERR] {}", line);
                }
            });
        }

        Self {
            cluster,
            running: true,
        }
    }

    fn stop(&mut self) {
        if !self.running {
            return;
        }

        println!("Stopping docker-compose cluster...");
        let _ = self.cluster.kill();
        let _ = self.cluster.wait();

        let output = Command::new("make")
            .args(["-C", "../", "down"])
            .output()
            .expect("Failed to run make down");

        if !output.stdout.is_empty() {
            println!(
                "[DOCKER-COMPOSE DOWN] {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        if !output.stderr.is_empty() {
            println!(
                "[DOCKER-COMPOSE DOWN ERROR] {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

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

    pub async fn send_bulk(
        &self,
        docs: &[serde_json::Value],
    ) -> mongodb::error::Result<Vec<serde_json::Value>> {
        if !docs.is_empty() {
            let docs: Vec<_> = docs
                .iter()
                .map(|v| {
                    let mut doc = mongodb::bson::to_document(v).unwrap();
                    let oid = mongodb::bson::oid::ObjectId::new();
                    doc.insert("_id", oid);
                    doc
                })
                .collect();
            self.collection.insert_many(docs.clone()).await?;
            // Convert a Vec<bson::Document> to Vec<serde_json::Value>
            let json_values: Vec<serde_json::Value> = docs
                .into_iter()
                .map(|doc| serde_json::to_value(doc).unwrap())
                .collect();

            println!("Inserted documents into MongoDB");
            return Ok(json_values);
        } else {
            println!("No documents to insert");
        }
        Ok(vec![])
    }

    pub async fn send_seq(
        &self,
        docs: &[serde_json::Value],
    ) -> mongodb::error::Result<Vec<serde_json::Value>> {
        if !docs.is_empty() {
            let docs: Vec<_> = docs
                .iter()
                .map(|v| {
                    let mut doc = mongodb::bson::to_document(v).unwrap();
                    let oid = mongodb::bson::oid::ObjectId::new();
                    doc.insert("_id", oid);
                    doc
                })
                .collect();
            for d in docs.iter() {
                self.collection.insert_one(d).await?;
            }
            // Convert a Vec<bson::Document> to Vec<serde_json::Value>
            let json_values: Vec<serde_json::Value> = docs
                .into_iter()
                .map(|doc| serde_json::to_value(doc).unwrap())
                .collect();

            println!("Inserted documents into MongoDB");
            return Ok(json_values);
        } else {
            println!("No documents to insert");
        }
        Ok(vec![])
    }
}

/// Consumer for receiving messages from RabbitMQ.
#[derive(Clone)]
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
        let consumer_tag = format!(
            "consumer_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let mut consumer = self
            .channel
            .basic_consume(
                &self.queue,
                consumer_tag.as_str(),
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

fn load_settings(config: &str, connections: &str) -> Settings {
    let config = config::Config::builder()
        .add_source(config::File::with_name(config))
        .build()
        .unwrap();

    let settings = config.try_deserialize::<Settings>().unwrap();

    let config = config::Config::builder()
        .add_source(config::File::with_name(connections))
        .build()
        .unwrap();
    let connections = config.try_deserialize::<Connections>().unwrap();

    let settings = Settings::new(connections, settings.collections().to_owned())
        .map_err(|e| anyhow::anyhow!("Failed to create settings: {}", e))
        .unwrap();
    settings
}

fn load_input_data(file: &str) -> Vec<serde_json::Value> {
    let file = std::fs::File::open(file).unwrap();
    serde_json::from_reader(file).unwrap()
}

async fn create_pubsub(settings: &Settings) -> (Client, Producer, Consumer) {
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

    let producer = Producer::new(client.clone(), &settings.collections()[0]).await;
    let consumer = Consumer::new(
        &settings.connections().rabbitmq_uri,
        &settings.collections()[0].rabbitmq.stream_name,
    )
    .await
    .unwrap();
    (client, producer, consumer)
}

async fn wait_for_metrics() {
    // Test metrics endpoint
    loop {
        let metrics_response = match reqwest::get("http://localhost:3000/metrics").await {
            Ok(response) => match response.text().await {
                Ok(text) => text,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        println!("Metrics response: {}", metrics_response);

        // Verify that metrics contain expected data
        // assert!(metrics_response.contains("mrcon_running_servers_total"));
        // assert!(metrics_response.contains("mrcon_collection_servers"));
        // assert!(metrics_response.contains("mrcon_tasks_started_total"));

        // Check if running total is 1, if so break the loop
        if metrics_response.contains("mrcon_running_servers_total 1") {
            break;
        }

        // Wait a bit before checking again
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test() {
    let mut cluster = Cluster::start();

    wait_for_metrics().await;

    let settings = load_settings(
        "tests/data/simple/config.yaml",
        "tests/data/simple/connections.yaml",
    );

    let input = load_input_data("tests/data/simple/input.json");

    let (client, producer, consumer) = create_pubsub(&settings).await;

    producer_consumer_send_in_bulk(producer.clone(), consumer.clone(), input.clone()).await;
    producer_consumer_send_one_by_one(producer, consumer, input).await;

    let health_response = reqwest::get("http://localhost:3000/health")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(health_response, "OK");

    // Test metrics endpoint
    let metrics_response = reqwest::get("http://localhost:3000/metrics")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    println!("Metrics response: {}", metrics_response);

    // Verify that metrics contain expected data
    assert!(metrics_response.contains("mrcon_running_servers_total"));
    assert!(metrics_response.contains("mrcon_collection_servers"));
    assert!(metrics_response.contains("mrcon_tasks_started_total"));

    watched_collection_is_dropped(client).await;

    cluster.stop();
}

async fn producer_consumer_send_in_bulk(
    producer: Producer,
    consumer: Consumer,
    input: Vec<serde_json::Value>,
) {
    let producer_task = tokio::spawn({
        let docs = input.clone();
        async move {
            println!("Sending {} documents to MongoDB", docs.len());
            let res = producer.send_seq(&docs).await;
            println!("Producer finished sending documents: {:?}", res);
            res
        }
    });

    let consumer_task = tokio::spawn(async move { consumer.receive_all(input.len()).await });

    let (sent, received) = tokio::try_join!(producer_task, consumer_task).unwrap();

    let mut full_docs = Vec::new();
    received.iter().enumerate().for_each(|(i, doc)| {
        if let Some(full) = doc.get("fullDocument").cloned() {
            println!("Document {} (fullDocument, no _id): {:?}", i, full);
            full_docs.push(full);
        }
    });

    assert_eq!(sent.unwrap(), full_docs);
}

async fn producer_consumer_send_one_by_one(
    producer: Producer,
    consumer: Consumer,
    input: Vec<serde_json::Value>,
) {
    let producer_task = tokio::spawn({
        let docs = input.clone();
        async move {
            println!("Sending {} documents to MongoDB", docs.len());
            let res = producer.send_seq(&docs).await;
            println!("Producer finished sending documents: {:?}", res);
            res
        }
    });

    let consumer_task = tokio::spawn(async move { consumer.receive_all(input.len()).await });

    let (sent, received) = tokio::try_join!(producer_task, consumer_task).unwrap();

    let mut full_docs = Vec::new();
    received.iter().enumerate().for_each(|(i, doc)| {
        if let Some(full) = doc.get("fullDocument").cloned() {
            println!("Document {} (fullDocument, no _id): {:?}", i, full);
            full_docs.push(full);
        }
    });
    assert_eq!(sent.unwrap(), full_docs);
}

async fn watched_collection_is_dropped(client: Client) {
    let docker = bollard::Docker::connect_with_socket_defaults().unwrap();

    let options = Some(bollard::query_parameters::InspectContainerOptions { size: false });
    let state = docker
        .inspect_container("connector", options.clone())
        .await
        .unwrap();
    assert!(state.state.unwrap().running.unwrap());
    client
        .database("test")
        .collection::<Document>("testcoll")
        .drop()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(5)).await;
    let state = docker
        .inspect_container("connector", options.clone())
        .await
        .unwrap();
    let exit_code = state.state.unwrap().exit_code.unwrap();
    assert_eq!(
        exit_code, 0,
        "Connector container exited with code {}",
        exit_code
    );
}
