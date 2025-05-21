# This Dockerfile should be built from the root of the repository.
# `docker build -f scripts/fly.dockerfile -t maf-server:latest .`

# TODO: cache the build better

FROM rust:1.86-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

ADD Cargo.lock ./
ADD crates/maf_container ./crates/maf_container
ADD crates/maf_container_host ./crates/maf_container_host
ADD wit ./wit

# Create Cargo.toml with correct workspaces
RUN echo '[workspace]\nmembers = ["crates/maf_container_host", "crates/maf_container_host/migrations", "crates/maf_container"]\nresolver="2"' > Cargo.toml
# Build the application
RUN cargo build --release --package maf_container_host

FROM debian:latest AS app

WORKDIR /app

RUN useradd -m -u 1000 maf_container_host

# Make bundle directory
RUN mkdir -p /app/bundle && \
    chown -R maf_container_host:maf_container_host /app/bundle
ENV BUNDLE_STORAGE_DIR=/app/bundle

# Install dependencies
RUN apt-get update && \
    apt-get install libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/maf_container_host /app/maf_container_host

EXPOSE 3000
USER maf_container_host
VOLUME /app/bundle

CMD [ "/app/maf_container_host" ]

