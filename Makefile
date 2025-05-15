# Makefile for mongodb-rabbitmq-connector workspace

.PHONY: all build build-release build-debug clean run check format help

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
	docker compose up

check:
	cargo check --workspace

format:
	cargo fmt --all

help:
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:' Makefile | grep -v '.PHONY' | sed 's/:.*//' | xargs -n1 echo ' -'
