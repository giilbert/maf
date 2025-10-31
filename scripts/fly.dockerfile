# This Dockerfile should be built from the root of the repository.
# `docker build -f scripts/fly.dockerfile -t cobble-server:latest .`

# TODO: cache the build better

FROM rust:1.86-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

ADD Cargo.lock ./
ADD crates/cobble_container ./crates/cobble_container
ADD crates/cobble_container_host ./crates/cobble_container_host
ADD crates/cobble_schemas ./crates/cobble_schemas

# Create Cargo.toml with correct workspaces
RUN echo '[workspace]\nmembers = ["crates/cobble_container_host", "crates/cobble_schemas", "crates/cobble_container_host/migrations", "crates/cobble_container"]\nresolver="2"' > Cargo.toml
# Build the application
RUN cargo build --release --package cobble_container_host

FROM debian:latest AS app

WORKDIR /app

RUN useradd -m -u 1000 cobble_container_host

# Make bundle directory
RUN mkdir -p /app/bundle && \
    chown -R cobble_container_host:cobble_container_host /app/bundle
ENV BUNDLE_STORAGE_DIR=/app/bundle
ENV ENVIRONMENT=production

# Install dependencies
RUN apt-get update && \
    apt-get install -y libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cobble_container_host /app/cobble_container_host

EXPOSE 1147
USER cobble_container_host
VOLUME /app/bundle

CMD [ "/app/cobble_container_host" ]

