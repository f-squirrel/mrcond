# Makefile for mongodb-rabbitmq-connector workspace

.PHONY: all build build-release build-debug clean run check format help venv test-integration

all: build-debug

build: build-debug

build-release:
	cargo build --workspace --release

build-debug:
	cargo build --workspace

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
	cargo test --workspace -- --nocapture

help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:' Makefile | grep -v '.PHONY' | sed 's/:.*//' | xargs -n1 echo ' -'
