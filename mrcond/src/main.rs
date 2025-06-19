//! Main entry point for the binary daemon
use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use mrcon::config::{Connections, Settings};
use mrcon::metrics::Metrics;
use mrcon::ConnectorServer;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// MongoDB-RabbitMQ Connector Daemon
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to config file (YAML)
    #[arg(short, long, default_value = "/app/config.yaml")]
    config: String,
    /// Prefix for environment variables
    #[arg(short, long, default_value = "MRCON")]
    prefix: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env(format!("{}_LOG", cli.prefix)))
        .init();

    let config = config::Config::builder()
        .add_source(config::Environment::default().prefix(&cli.prefix))
        .build()?;
    let connections = config.try_deserialize::<Connections>()?;

    let config = config::Config::builder()
        .add_source(config::File::with_name(&cli.config))
        .build()?;

    let settings = config.try_deserialize::<Settings>()?;

    let settings = Settings::new(connections, settings.collections().to_owned())
        .map_err(|e| anyhow::anyhow!("Failed to create settings: {}", e))?;

    // Create shared metrics instance
    let metrics = Metrics::new();
    let metrics_for_server = metrics.clone();
    let metrics_for_api = metrics.clone();

    let health_api = tokio::spawn(async move {
        async fn health() -> &'static str {
            "OK"
        }

        let metrics_clone = metrics_for_api.clone();
        let metrics_handler = {
            let metrics = metrics_clone;
            move || async move {
                metrics
                    .export()
                    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        };

        let app = Router::new()
            .route("/health", get(health))
            .route("/metrics", get(metrics_handler));
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        axum::serve(listener, app).await
    });

    let server = tokio::spawn(async move {
        let server = ConnectorServer::with_metrics(settings, metrics_for_server);
        server.serve().await
    });

    let (health_res, server_res) = tokio::try_join!(health_api, server)?;
    health_res?;
    server_res?;
    Ok(())
}
