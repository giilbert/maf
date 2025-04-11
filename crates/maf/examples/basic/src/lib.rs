use std::time::Duration;

use maf::{self, tasks, App, Body, Channel, User};

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

async fn on_connect(user: User) {
    println!("user connected!");

    // TODO: less hacky way to get app
    let rx_channel = Channel::<String>::new(user.state.clone(), "hello");
    tasks::spawn(async move {
        loop {
            let message = rx_channel.recv().await.expect("failed to receive");
            println!("channel: {message:?}");
        }
    });

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
