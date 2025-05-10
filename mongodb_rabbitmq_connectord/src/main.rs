//! Main entry point for the binary daemon
use clap::Parser;
use mongodb_rabbitmq_connector::config::{Collection, Connections, Settings};
use mongodb_rabbitmq_connector::ConnectorServer;

/// MongoDB-RabbitMQ Connector Daemon
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to config file (YAML)
    #[arg(short, long, default_value = "/app/config.yaml")]
    config: String,
    /// Prefix for environment variables
    #[arg(short, long, default_value = "MRQCONN")]
    prefix: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let config = config::Config::builder()
        .add_source(config::File::with_name(&cli.config))
        .build()
        .unwrap();

    let collections = config.try_deserialize::<Vec<Collection>>().unwrap();

    let config = config::Config::builder()
        .add_source(config::Environment::default().prefix(&cli.prefix))
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
