# syntax=docker/dockerfile:1

# Comments are provided throughout this file to help you get started.
# If you need more help, visit the Dockerfile reference guide at
# https://docs.docker.com/engine/reference/builder/

################################################################################
# Create a stage for building the application.

ARG RUST_VERSION=1.75.0
FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /app

# Build the application.
# Leverage a cache mount to /usr/local/cargo/registry/
# for downloaded dependencies and a cache mount to /app/target/ for 
# compiled dependencies which will speed up subsequent builds.
# Leverage a bind mount to the ml_processor, rest_server, and shared
# directories to avoid having to copy the
# source code into the container. Once built, copy the executable to an
# output directory before the cache mounted /app/target is unmounted.
RUN --mount=type=bind,source=ml_processor,target=ml_processor \
    --mount=type=bind,source=rest_server,target=rest_server \
    --mount=type=bind,source=shared,target=shared \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    set -e; \
    cargo build --release; \
    cp target/release/rest_server /bin/rest_server; \
    cp target/release/ml_processor /bin/ml_processor

################################################################################
FROM debian:bookworm-slim AS template

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#user
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

################################################################################
# Create a new stage for running rest_server that contains the minimal
# runtime dependencies for the application. This often uses a different base
# image from the build stage where the necessary files are copied from the build
# stage.
FROM template AS rest_server

# Copy the executable from the "build" stage.
COPY --from=build /bin/rest_server /bin/

# Expose the port that the application listens on.
EXPOSE 3000

# What the container should run when it is started.
CMD ["/bin/rest_server"]

################################################################################
# Create a new stage for installing `aria2c`.
FROM debian:bookworm-slim AS aria

RUN apt update && apt install -y aria2

################################################################################
# Create a new stage for running ml_processor.
FROM template AS ml_processor

# Get `aria2c` for downloading the model.
COPY --from=aria /usr/bin/aria2c /usr/bin/aria2c

# Copy the executable from the "build" stage.
COPY --from=build /bin/ml_processor /bin/

# What the container should run when it is started.
CMD ["/bin/ml_processor"]
