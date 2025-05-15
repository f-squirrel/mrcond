//! Main entry point for the binary daemon
use anyhow::Result;
use clap::Parser;
use mongodb_rabbitmq_connector::config::{Connections, Settings};
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
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let config = config::Config::builder()
        .add_source(config::Environment::default().prefix(&cli.prefix))
        .build()?;
    let connections = config.try_deserialize::<Connections>()?;

    let config = config::Config::builder()
        .add_source(config::File::with_name(&cli.config))
        .build()?;

    let settings = config.try_deserialize::<Settings>()?;

    let settings = Settings {
        connections,
        collections: settings.collections,
    };
    let server = ConnectorServer::new(settings);
    server.serve().await?;
    Ok(())
}
