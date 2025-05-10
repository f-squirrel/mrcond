# syntax=docker/dockerfile:1
FROM rust:1.86 as builder

WORKDIR /app
COPY . .
RUN cargo build --release -p mongodb_rabbitmq_connectord

FROM ubuntu:24.04

WORKDIR /app
COPY --from=builder /app/target/release/mongodb_rabbitmq_connectord /app/mongodb_rabbitmq_connectord
COPY ./config.yaml /app/config.yaml
CMD ["/app/mongodb_rabbitmq_connectord", "/app/config.yaml"]
