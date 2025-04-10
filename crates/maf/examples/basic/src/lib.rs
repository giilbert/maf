use std::time::Duration;

use maf::{self, serde_json::json, tasks, App, Body, User};

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

async fn on_connect(user: User) {
    println!("user connected!");

    loop {
        let _ = user.channel("hello").send("Hello, world!");
        tasks::sleep(Duration::from_secs(1)).await;
    }
}

fn build() -> App {
    let app = App::new()
        .on_connect(on_connect)
        .add_rpc_function("test", test_rpc);

    app
}

maf::register_build!(build);
