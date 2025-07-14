use std::fs::File;
use std::io::prelude::*;

use crate::{input::input, pretty};

pub async fn handle_init(project_name: Option<String>) -> anyhow::Result<()> {
    match project_name {
        Some(name) => {
            pretty::info!("Creating new project: {}", name.bold());
            run_setup_commands(name).await
        }
        None => {
            pretty::info!("Creating new project with a random name");
            let name = input!(
                transform: |name: String| {
                    if name.is_empty() {
                        anyhow::bail!("Name cannot be empty.")
                    }
                    if name.len() > 100 {
                        anyhow::bail!("Name cannot be longer than 100 characters.")
                    }
                    if !name
                        .chars()
                        .all(|c| (c.is_ascii_alphanumeric() && (c.is_ascii_lowercase() || c.is_numeric())) || c == '-')
                    {
                        anyhow::bail!("Name can only contain lowercase alphanumeric characters and hyphens.")
                    }

                    Ok(name)
                },
                "{} {}:",
                "Name".bold(),
                "(Lowercase alphanumeric characters and hyphens)".dimmed()
            );
            run_setup_commands(name).await
        }
    }
}

pub async fn run_setup_commands(project_name: String) -> anyhow::Result<()> {
    pretty::info!(
        "running setup command for project `{}`",
        project_name.bold()
    );

    println!("\n----------\n");

    let start = std::time::Instant::now();

    let mut rustup_target_process = tokio::process::Command::new("rustup")
        .arg("rustup target add wasm32-wasip2")
        .spawn()?;
    rustup_target_process.wait().await?;

    let mut cargo_new_process = tokio::process::Command::new("cargo")
        .arg("new --lib server")
        .spawn()?;
    cargo_new_process.wait().await?;

    let mut maf_project_toml = File::create("server/maf-project.toml")?;
    maf_project_toml.write_all(
        format!(
            "name = \"{project_name}\" # Edit this to your application name
rooms = \"AutoCreate\" # This will make MAF put all users in a single *room*

# If your build output path is different, adjust the paths accordingly.
[debug]
command = \"cargo build --target wasm32-wasip2\"
output = \"./target/wasm32-wasip2/debug/server.wasm\"

[release]
command = \"cargo build --target wasm32-wasip2 --release\"
output = \"./target/wasm32-wasip2/release/server.wasm\"",
        )
        .as_bytes(),
    )?;

    let mut cargo_toml = File::options().append(true).open("server/Cargo.toml")?;
    cargo_toml.write_all(
        b"maf = \"0.1.0\"
# To use the latest version, you can use the following instead:
# maf = { repository = \"https://github.com/giilbert/maf\" }
# serde is a library for serializing and deserializing data
serde = { version = \"^1\", features = [\"derive\"] }

[lib]
crate-type = [\"cdylib\"] # This is required for Cargo to build the library",
    )?;

    let mut lib_rs = File::create("server/src/lib.rs")?;
    lib_rs.write_all(
        b"use maf::*;

struct CounterStore;

impl StoreData for CounterStore {
    type Data = i32;

    fn init() -> Self::Data {
        0
    }

    // Determines what data to send to the client when the store is serialized
    fn select(data: &Self::Data, _user: &User) -> impl serde::Serialize {
        data
    }

    // This name will be used to identify the store
    fn name() -> impl AsRef<str> {
        \"count\"
    }
}

// RPC functions can be used to perform actions on the server
async fn increment_counter(
    // Special types for extracting parameters, data, and context
    Params(counter): Params<i32>,
    test: Store<CounterStore>
) -> i32 {
    let mut data = test.write().await;
    *data += counter;
    println!(\"incremented counter by {counter}. new value: {}\", &*data);
    *data
}

async fn on_connect(user: User) {
    println!(\"user connected! id: {}\", user.meta.id());
}

// Declare what the MAF application should do
fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .rpc(\"increment_counter\", increment_counter)
        .build()
}

maf::register!(build);",
    )?;

    println!("\n----------\n");

    pretty::info!("build completed in {:.2?}", start.elapsed());

    Ok(())
}
