//! Main entry point for the binary daemon
use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use mongodb_rabbitmq_connector::config::{Connections, Settings};
use mongodb_rabbitmq_connector::ConnectorServer;
use std::net::SocketAddr;

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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let health_api = tokio::spawn(async move {
        async fn health() -> &'static str {
            "OK"
        }
        let app = Router::new().route("/health", get(health));
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await
    });

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

    let server = tokio::spawn(async move {
        let server = ConnectorServer::new(settings);
        server.serve().await
    });

    let (health_res, server_res) = tokio::try_join!(health_api, server)?;
    health_res?;
    server_res?;
    Ok(())
}
