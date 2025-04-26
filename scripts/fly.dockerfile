# This Dockerfile should be built from the root of the repository.
# `docker build -f scripts/fly.dockerfile -t maf-container-fly:latest .`

# TODO: cache the build better

FROM rust:1.86-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

ADD Cargo.lock ./
ADD crates/maf_container ./crates/maf_container
ADD wit ./wit

# Create Cargo.toml with correct workspaces
RUN echo '[workspace]\nmembers = ["crates/maf_container", "crates/maf_container/migrations"]\nresolver="2"' > Cargo.toml
# Build the application
RUN cargo build --release --package maf_container

FROM debian:latest AS app

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/maf_container maf_container

EXPOSE 3000

ENV RUST_LOG=info

ENTRYPOINT [ "/app/maf_container" ]

