# Makefile for mongodb-rabbitmq-connector workspace

.PHONY: all build build-release build-debug clean run check format help venv test test-integration test-integration-quiet test-metrics

all: build-debug

build: build-debug

build-release:
	cargo build --workspace --release

build-debug:
	cargo build --workspace

build-docker-release:
	docker build -t mrcond:latest -f ./mrcond/Dockerfile.release .
clean:
	cargo clean

down:
	docker compose down

run:
	docker compose up --build

check:
	cargo check --workspace

check-clippy:
	cargo clippy --workspace -- -D warnings

clippy:
	cargo clippy --workspace --fix

format:
	cargo fmt --all

check-format:
	cargo fmt --all --check

test:
	cargo test --workspace -- --nocapture --test-threads=1

test-integration:
	@echo "Running integration tests with docker-compose output..."
	cargo test --workspace --test e2e -- --nocapture

test-integration-quiet:
	@echo "Running integration tests (quiet mode)..."
	RUST_LOG=warn cargo test --workspace --test e2e -- --nocapture 2>/dev/null

run-logs:
	docker compose up --build

logs:
	docker compose logs -f

logs-connector:
	docker compose logs -f connector

logs-mongo:
	docker compose logs -f mongodb

logs-rabbitmq:
	docker compose logs -f rabbitmq

test-metrics:
	@echo "Running metrics endpoint tests..."
	@chmod +x test-metrics.sh
	./test-metrics.sh

help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:' Makefile | grep -v '.PHONY' | sed 's/:.*//' | xargs -n1 echo ' -'
