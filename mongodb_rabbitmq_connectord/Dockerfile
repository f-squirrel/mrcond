# syntax=docker/dockerfile:1
FROM ubuntu:24.04

WORKDIR /app
COPY ./config.yaml /app/config.yaml
COPY ../target/release/mongodb_rabbitmq_connectord /app/mongodb_rabbitmq_connectord
CMD ["/app/mongodb_rabbitmq_connectord", "/app/config.yaml"]
