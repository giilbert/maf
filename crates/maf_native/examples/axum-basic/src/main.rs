use axum::{extract::Path, routing::get};
use maf_native::{axum::Room, prelude::*};

fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta.id());
}

fn app() -> AppBuilder {
    App::builder()
        .on_connect(on_connect)
        .background(|app: App| async move {
            println!("hello world!");
        })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let room = Room::new(app())?;

    let router = axum::Router::new().route(
        "/connect",
        get(|params: Path<(String, String, String)>| async move { "" }),
    );

    Ok(())
}
