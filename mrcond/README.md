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

The daemon is configured via YAML files that specify which MongoDB collections to watch and how to forward their change events to RabbitMQ.

### Simple Configuration

For basic use cases with default settings:

```yaml
collections:
  - watched:
      db_name: "myapp"
      coll_name: "users"
      change_stream_pre_and_post_images: false
    resume_tokens:
      tokens_db_name: "myapp"
      tokens_coll_name: "resume_tokens"
      tokens_coll_capped: true
      tokens_coll_size_in_bytes: 1048576
    rabbitmq:
      queue:
        name: "user_changes"
```

### Advanced Configuration

For production environments with custom exchange, queue settings, and message properties:

```yaml
collections:
  - watched:
      db_name: "myapp"
      coll_name: "users"
      change_stream_pre_and_post_images: true
    resume_tokens:
      tokens_db_name: "myapp"
      tokens_coll_name: "resume_tokens"
      tokens_coll_capped: true
      tokens_coll_size_in_bytes: 10485760  # 10MB
    rabbitmq:
      exchange:
        name: "myapp_events"
        type: "topic"
        declare_options:
          durable: true
          auto_delete: false
      queue:
        name: "testqueue"
        bind_options:
          nowait: false
        declare_options:
          passive: false
          durable: false
          exclusive: false
          auto_delete: false
          nowait: false
        basic_publish_options:
          mandatory: false
          immediate: false
        basic_properties:
          persistent: false
          content_type: "application/json"
          content_encoding: "utf-8"
          priority: 0
          correlation_id: ""
          reply_to: ""
          expiration: ""
          message_id: ""
          timestamp: 0
          user_id: ""
          app_id: ""
          cluster_id: ""
      routing_key: "test_routing_key"
```

### Configuration Reference

- **collections**: List of collections to watch and forward to RabbitMQ.
  - **watched**: MongoDB collection to watch.
    - `db_name`: Database name.
    - `coll_name`: Collection name.
    - `change_stream_pre_and_post_images`: Enable pre/post images for updates (bool).
  - **resume_tokens**: Where to store resume tokens for reliable streaming.
    - `tokens_db_name`: Database for tokens.
    - `tokens_coll_name`: Collection for tokens.
    - `tokens_coll_capped`: Use capped collection (bool).
    - `tokens_coll_size_in_bytes`: Size for capped collection in bytes.
  - **rabbitmq**: Target RabbitMQ configuration.
    - **exchange** (optional): Exchange configuration.
      - `name`: Exchange name.
      - `type`: Exchange type (`direct`, `topic`, `fanout`, `headers`).
      - **declare_options**: Exchange declaration options (all optional, default false).
        - `durable`: Survive broker restart.
        - `auto_delete`: Delete when no queues bound.
        - `internal`: Internal exchange.
    - **queue**: Queue configuration.
      - `name`: Queue name.
      - **declare_options**: Queue declaration options (all optional, default false).
        - `durable`: Survive broker restart.
        - `exclusive`: Only accessible by this connection.
        - `auto_delete`: Delete when no consumers.
      - **bind_options**: Queue binding options (optional).
        - `nowait`: Don't wait for bind confirmation.
      - **basic_publish_options**: Publishing options (optional).
        - `mandatory`: Return message if unroutable.
        - `immediate`: Return message if no consumers.
      - **basic_properties**: Message properties (all optional).
        - `content_type`: MIME type (e.g., "application/json").
        - `content_encoding`: Content encoding (e.g., "gzip", "utf-8").
        - `delivery_mode`: 1 (transient) or 2 (persistent).
        - `priority`: Message priority (0-9).
        - `correlation_id`: For request-response patterns.
        - `reply_to`: Reply queue name.
        - `expiration`: Message TTL in milliseconds.
        - `message_id`: Unique message identifier.
        - `timestamp`: Message timestamp (Unix timestamp).
        - `user_id`: User identifier.
        - `app_id`: Application identifier.
    - `routing_key` (optional): Routing key for exchange routing.

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
