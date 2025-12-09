default:
    @just --list

# deploy the platform host to fly.io without high availability
deploy:
    fly deploy --ha=false -c scripts/fly.toml

docker-build:
    docker build -f scripts/fly.dockerfile -t maf-server:latest .

# build and run maf_cli in development mode
[working-directory: "."]
dev-cli *args:
    cargo build --bin maf_cli
    cd "{{invocation_directory()}}" && {{justfile_directory()}}/target/debug/maf_cli "{{args}}"
