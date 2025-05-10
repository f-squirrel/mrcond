//! Main entry point for the binary daemon
use mongodb_rabbitmq_connector::config::Settings;
use mongodb_rabbitmq_connector::ConnectorServer;
use tracing_subscriber;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt::init();
    let settings = match Settings::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!(error = %e, "Failed to load config");
            std::process::exit(1);
        }
    };
    let server = ConnectorServer::new(settings);
    if let Err(e) = server.serve().await {
        tracing::error!(error = %e, "Connector server failed");
        std::process::exit(1);
    }
}
