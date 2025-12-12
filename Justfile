# shows the available just commands
default:
    @just --list

# deploy the platform host to fly.io without high availability
deploy:
    fly deploy --ha=false -c scripts/fly.toml

# build the maf-server docker image
docker-build:
    docker build -f scripts/fly.dockerfile -t maf-server:latest .

# build and run maf_cli in development mode
[working-directory: "."]
dev-cli *args:
    cargo build --bin maf_cli
    cd "{{invocation_directory()}}" && {{justfile_directory()}}/target/debug/maf_cli "{{args}}"

# builds @usemaf/react and @usemaf/client
[group('npm')]
npm-build:
    pnpm run --filter=@usemaf/react --filter=@usemaf/client build

# builds @usemaf/react and @usemaf/client and publishes them to npm
[group('npm')]
npm-publish: npm-build
    pnpm publish --filter=@usemaf/react --filter=@usemaf/client --access public --no-git-checks

# [interactive] bumps the version of all packages 
[group('npm')]
npm-bump *args:
    bun run scripts/bump.ts {{args}}
