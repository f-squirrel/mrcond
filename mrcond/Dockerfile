# syntax=docker/dockerfile:1
FROM ubuntu:24.04

WORKDIR /app
COPY ./config.yaml /app/config.yaml
CMD ["/app/mrcond", "--config", "/app/config.yaml"]
