# syntax=docker/dockerfile:1
FROM ubuntu:24.04

WORKDIR /app
COPY ./config.yaml /app/config.yaml
CMD ["/app/mongodb_rabbitmq_connectord", "--config", "/app/config.yaml"]
