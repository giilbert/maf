use std::time::Duration;

use maf::*;

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

async fn on_connect(app: App, user: User) {
    println!("user connected!");

    let rx_channel = app.channel::<String>("hello");
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
    App::builder()
        .on_connect(on_connect)
        .rpc("test", test_rpc)
        .build()
}

maf::register!(build);
