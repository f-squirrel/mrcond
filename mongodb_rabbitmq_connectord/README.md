# mongodb_rabbitmq_connectord

A production-ready CLI daemon for streaming MongoDB change events to RabbitMQ, built on the `mongodb_rabbitmq_connector` library.

## Features

- **CLI Configuration**: Select config file and environment variable prefix via command-line arguments.
- **Robust Supervision**: Supervises connector jobs for each collection, with automatic restart and retry logic.
- **Structured Logging**: Uses `tracing` for structured, per-crate logging. Control log level with `RUST_LOG`.
- **Docker-Ready**: Designed for containerized deployment and integration testing with Docker Compose.
- **Healthcheck Endpoint**: (Stubbed) for future readiness and observability.

## Usage

Build and run the daemon with your configuration:

```sh
cargo build --release
./target/release/mongodb_rabbitmq_connectord --config config.yaml --prefix MYAPP_
```

Or use Docker Compose for local integration testing:

```sh
make run
```

## Command-Line Options

- `--config <FILE>`: Path to the YAML configuration file (default: `config.yaml`).
- `--prefix <PREFIX>`: Environment variable prefix for config overrides (default: `MRQCONN`).

## Configuration

- See the main repo `config.yaml` for an example.
- All settings can be overridden via environment variables with the specified prefix.

## Development & Testing

- Use the provided `Makefile` for build, test, and Docker Compose integration.
- Run `make run` to start MongoDB and RabbitMQ in Docker and launch the daemon.
- Logs are controlled via the `RUST_LOG` environment variable.

## License

Licensed under the MIT License. See the main repository for details.
