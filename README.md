[![Test Suite](https://github.com/f-squirrel/mrcond/actions/workflows/ci.yml/badge.svg)](https://github.com/f-squirrel/mrcond/actions/workflows/ci.yml)

# MongoDB-RabbitMQ Connector

This repository contains a production-ready system for streaming MongoDB change events to RabbitMQ, featuring a robust library and a CLI daemon for deployment.

## Crates

- [`mrcon`](./mrcon/README.md): Core library for MongoDB-to-RabbitMQ streaming, resume tokens, and connector logic.
- [`mrcond`](./mrcond/README.md): CLI daemon for running the connector as a supervised service, with healthcheck and Docker support.

## Quick Start

1. See [`mrcond/README.md`](./mrcond/README.md) for CLI usage, configuration, and deployment instructions.
2. See [`mrcon/README.md`](./mrcon/README.md) for library API documentation and integration details.

## Development

- Use the provided `Makefile` for building, testing, and running integration tests with Docker Compose.
- Each crate is self-documented; see their respective README files for details.

## License

Licensed under the MIT License. See each crate for details.
