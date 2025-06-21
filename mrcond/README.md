# mrcond

A production-ready CLI daemon for streaming MongoDB change events to RabbitMQ, built on the `mrcon` library.

## Features

- **Streams MongoDB change events to RabbitMQ** with robust, resumable delivery.
- **Supervises jobs for each collection** with automatic restart and retry logic.
- **Configurable via YAML, environment variables, and CLI**.
- **Structured logging and tracing** (configurable via env vars).
- **Healthcheck endpoint** at `/health` (returns HTTP 200 OK).
- **Docker-ready** for containerized deployment and integration testing.

## Usage

Build and run the daemon with your configuration:

```sh
cargo build --release
./target/release/mrcond --config config.yaml --prefix MYAPP
```

Or use Docker Compose for local integration testing:

```sh
make run
```

## Command-Line Options

```plain
MongoDB-RabbitMQ Connector Daemon

Usage: mrcond [OPTIONS]

Options:
  -c, --config <CONFIG>  Path to config file (YAML) [default: /app/config.yaml]
  -p, --prefix <PREFIX>  Prefix for environment variables [default: MRCON]
  -h, --help             Print help
  -V, --version          Print version
```

## Configuration

### MongoDB collection and rabbitmq stream settings config file

Example:

```yaml
collections:
  - watched:
      db_name: "test"
      coll_name: "testcoll"
      change_stream_pre_and_post_images: false
    resume_tokens:
      tokens_db_name: "test"
      tokens_coll_name: "resume_tokens"
      tokens_coll_capped: true
      tokens_coll_size_in_bytes: 1048576
    rabbitmq:
      stream_name: "testqueue"
```

- **collections**: List of collections to watch and forward to RabbitMQ.
  - **watched**: MongoDB collection to watch.
    - `db_name`: Database name.
    - `coll_name`: Collection name.
    - `change_stream_pre_and_post_images`: Enable pre/post images (bool).
  - **resume_tokens**: Where to store resume tokens for reliable streaming.
    - `tokens_db_name`: Database for tokens.
    - `tokens_coll_name`: Collection for tokens.
    - `tokens_coll_capped`: Use capped collection (bool).
    - `tokens_coll_size_in_bytes`: Size for capped collection.
  - **rabbitmq**: Target RabbitMQ queue/stream.
    - `stream_name`: Queue/stream name.

### Environment Variables

You can set MongoDB and RabbitMQ connection strings, as well as log level, using environment variables with the configured prefix (default: `MRCON`).

Examples:

```sh
# Set MongoDB connection string
MRCON_MONGO_URI="mongodb://mongodb:27017/"

# Set RabbitMQ connection string
MRCON_RABBITMQ_URI="amqp://guest:guest@rabbitmq:5672/my_vhost"

# Set log level (info, debug, warn, etc.)
MRCON_LOG="info"
```

- The prefix is set by `--prefix` (default: `MRCON`).

## Health check

A health endpoint is available at `http://localhost:3000/health` and returns HTTP 200 OK.

## Metrics

Prometheus metrics are exposed at `http://localhost:3000/metrics` for monitoring and observability.

## Credits

This tool is inspired by [mongodb-nats-connector](https://github.com/damianiandrea/mongodb-nats-connector).

## License

Licensed under the MIT License. See the main repository for details.
