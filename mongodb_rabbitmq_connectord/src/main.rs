//! Main entry point for the binary daemon
use config::Config;
use mongodb_rabbitmq_connector::config::{Collection, Connections, Settings};
use mongodb_rabbitmq_connector::ConnectorServer;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = Config::builder()
        .add_source(config::File::with_name("/app/config.yaml"))
        .build()
        .unwrap();

    let collections = config.try_deserialize::<Vec<Collection>>().unwrap();

    let config = Config::builder()
        .add_source(config::Environment::default().prefix("MRQCONN"))
        .build()
        .unwrap();

    let connections = config.try_deserialize::<Connections>().unwrap();

    let settings = Settings {
        connections,
        collections,
    };

    let server = ConnectorServer::new(settings);
    if let Err(e) = server.serve().await {
        tracing::error!(error = %e, "Connector server failed");
        std::process::exit(1);
    }
}
