# shows the available just commands
default:
    @just --list

_start-db:
    docker compose up -d

# deploy the platform host to fly.io without high availability
deploy:
    fly deploy --ha=false -c scripts/fly.toml

# runs pgcli for the local development database
pgcli: _start-db
    pgcli postgres://postgres:ferris@localhost:5432

# build the maf-server docker image
docker-build:
    docker build -f scripts/fly.dockerfile -t maf-server:latest .

# start the platform server in development mode
dev-platform: _start-db
    RUST_LOG=info,maf_platform_host=debug,maf_core=trace cargo run --bin maf_platform_host

# apply the schema migrations
migrate: _start-db
    cargo run --package migrations up --verbose

# build and run maf_cli in development mode
[working-directory(".")]
dev-cli *args:
    cargo build --bin maf_cli
    cd "{{ invocation_directory() }}" && {{ justfile_directory() }}/target/debug/maf_cli {{ args }}

# install maf_cli binary to cargo bin directory
install-cli:
    cargo install --path crates/maf_cli

# starts a development server that watches for changes in packages/react and packages/client and rebuilds them on change
[group('npm')]
watch:
    pnpm --parallel --filter=@usemaf/react --filter=@usemaf/client --filter=@usemaf/platform dev

# checks types in @usemaf/react and @usemaf/client
[group('npm')]
type-check:
    pnpm --parallel --filter=@usemaf/react --filter=@usemaf/client --filter=@usemaf/platform type-check

# builds @usemaf/react and @usemaf/client
[group('npm')]
npm-build:
    pnpm run --filter=@usemaf/react --filter=@usemaf/client --filter=@usemaf/platform build

# builds @usemaf/react and @usemaf/client and publishes them to npm
[group('npm')]
npm-publish: npm-build
    pnpm publish --filter=@usemaf/react --filter=@usemaf/client --filter=@usemaf/platform --access public --no-git-checks

# [interactive] bumps the version of all packages
[group('npm')]
npm-bump *args:
    bun run scripts/bump.ts {{ args }}
