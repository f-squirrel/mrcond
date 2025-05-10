//! Main entry point for the binary daemon
use config::Config;
use mongodb_rabbitmq_connector::config::Settings;
use mongodb_rabbitmq_connector::ConnectorServer;
use tracing_subscriber;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = Config::builder()
        .add_source(config::File::with_name("/app/config.yaml"))
        .build()
        .unwrap();

    let settings = config.try_deserialize::<Settings>().unwrap();

    let server = ConnectorServer::new(settings);
    if let Err(e) = server.serve().await {
        tracing::error!(error = %e, "Connector server failed");
        std::process::exit(1);
    }
}
