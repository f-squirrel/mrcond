# mongodb_rabbitmq_connector

A robust, modular Rust library for streaming MongoDB change events to RabbitMQ, with persistent resume tokens, structured logging, and production-grade error handling.

## Features

- **MongoDB Change Streams**: Streams real-time change events from MongoDB collections.
- **RabbitMQ Integration**: Publishes change events to RabbitMQ queues.
- **Persistent Resume Tokens**: Stores resume tokens in MongoDB for reliable, resumable streaming.
- **Configurable**: Supports configuration via YAML files and environment variables.
- **Structured Logging**: Uses `tracing` for per-crate, structured logs.
- **Supervision & Resilience**: Automatic job restart and retry logic for robust operation.

## Usage

This crate is intended to be used as a library by the `mongodb_rabbitmq_connectord` binary, but can be integrated into other Rust applications as well.

### Example

```rust
// TODO(DD)
```

## Configuration

- See `config.yaml` for an example configuration file.
- All settings can be overridden via environment variables with a prefix (e.g., `MYAPP_`).

## Development & Testing

- Use the provided `Makefile` for build, test, and Docker Compose integration.
- Run `make run` to start MongoDB and RabbitMQ in Docker and launch the connector.
- Logs are controlled via the `RUST_LOG` environment variable.
