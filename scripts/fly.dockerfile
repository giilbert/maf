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

RUN useradd -m -u 1000 maf_container

# Make bundle directory
RUN mkdir -p /app/bundle && \
    chown -R maf_container:maf_container /app/bundle
ENV BUNDLE_STORAGE_DIR=/app/bundle

# Install dependencies
RUN apt-get update && \
    apt-get install libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/maf_container /app/maf_container

EXPOSE 3000
USER maf_container

CMD [ "/app/maf_container" ]

