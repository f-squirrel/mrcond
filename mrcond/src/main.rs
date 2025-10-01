//! Main entry point for the binary daemon
use anyhow::Result;
use axum::{Router, routing::get};
use clap::Parser;
use mrcon::ConnectorServer;
use mrcon::config::{Connections, Settings};
use mrcon::metrics::Metrics;

use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

const HEALTH_ENDPOINT: &str = "/health";
const METRICS_ENDPOINT: &str = "/metrics";

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

    tracing::trace!("Settings: {:?}", settings.collections());

    // Create shared metrics instance
    let metrics = Metrics::new();
    let metrics_for_server = metrics.clone();
    #[cfg(feature = "metrics")]
    let metrics_for_api = metrics.clone();

    let health_api = tokio::spawn(async move {
        async fn health() -> &'static str {
            "OK"
        }

        #[cfg(feature = "metrics")]
        let metrics_handler = {
            let metrics = metrics_for_api;
            move || async move {
                metrics
                    .export()
                    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        };

        #[cfg(feature = "metrics")]
        let app = Router::new()
            .route(HEALTH_ENDPOINT, get(health))
            .route(METRICS_ENDPOINT, get(metrics_handler));

        #[cfg(not(feature = "metrics"))]
        let app = Router::new().route(HEALTH_ENDPOINT, get(health));
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        axum::serve(listener, app).await
    });

    let server = tokio::spawn(async move {
        let server = ConnectorServer::with_metrics(settings, metrics_for_server);
        server.serve().await
    });

    tokio::select! {
        health_result = health_api => {
            let result = health_result?;
            info!("Health API server exited, shutting down");
            result.map_err(Into::into)
        }
        server_result = server => {
            let result = server_result?;
            info!("Connector server exited, shutting down");
            result.map_err(Into::into)
        }
    }
}
