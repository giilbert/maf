# This Dockerfile should be built from the root of the repository.
# `just docker-build` or `docker build -f scripts/fly.dockerfile -t maf-server:latest .`

FROM rust:1.91-slim-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

ADD Cargo.lock ./
ADD crates/maf_platform ./crates/maf_platform_host
ADD crates/maf_platform_host ./crates/maf_platform_host
ADD crates/maf_schemas ./crates/maf_schemas
ADD crates/maf/wit ./crates/maf/wit

# Create Cargo.toml with correct workspaces
RUN echo '[workspace]\nmembers = ["crates/maf_platform_host", "crates/maf_schemas", "crates/maf_platform_host/migrations", "crates/maf_platform"]\nresolver="2"' > Cargo.toml
# Build maf_platform_host with caching on target and cargo registry (packages)
RUN \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --package maf_platform_host && \
    cp ./target/release/maf_platform_host /app/maf_platform_host

FROM debian:latest AS app

WORKDIR /app

RUN useradd -m -u 1000 maf_platform_host

# Make bundle directory
RUN mkdir -p /app/bundle && \
    chown -R maf_platform_host:maf_platform_host /app/bundle
ENV BUNDLE_STORAGE_DIR=/app/bundle
ENV ENVIRONMENT=production

# Install dependencies
RUN apt-get update && \
    apt-get install -y libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/maf_platform_host /app/maf_platform_host

EXPOSE 1147
USER maf_platform_host
VOLUME /app/bundle

CMD [ "/app/maf_platform_host" ]

