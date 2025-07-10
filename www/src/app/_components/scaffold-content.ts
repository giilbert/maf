export const CODE_BLOCKS = {
  "Rust:src/lib.rs": {
    language: "rust",
    content: `
// [1] Import MAF server library
use maf::*;

fn build() -> App {
  println!("Hello, world!");

  App::builder()
    // [2] Declare application functionality
    .rpc("greet", |Params(name): Params<String>| {
        format!("Hello, {}!", name)
    })
    .build()
}

// [3] Register the application to be ran
maf::register!(build);
`,
  },
  "Rust:maf.toml": {
    language: "toml",
    content: `
# Configuration file for MAF applications
name = "your-app-name"

[debug]
command = "cargo build --target wasm32-wasip2"
output = "./target/wasm32-wasip2/debug/demo.wasm"

[release]
command = "cargo build --target wasm32-wasip2 --release"
output = "./target/wasm32-wasip2/release/demo.wasm"
`,
  },
  "Rust:Cargo.toml": {
    language: "toml",
    content: `
[package]
name = "your-app-name"
version = "0.1.0"
edition = "2024"

[dependencies]
maf = "^1"
serde = { version = "^1", features = ["derive"] }

[lib]
crate-type = ["cdylib"]

`,
  },
} as const;
