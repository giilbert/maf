default:
    @just --list

# deploy the platform host to fly.io without high availability
deploy:
    fly deploy --ha=false -c scripts/fly.toml

docker-build:
    docker build -f scripts/fly.dockerfile -t maf-server:latest .

# build and run maf_cli in development mode
dev-cli *args:
    cargo run --bin maf_cli -- {{args}}
