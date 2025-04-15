use std::time::Duration;

use maf::*;

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

async fn on_connect(user: User) {
    println!("user connected!");

    // let mut hello_channel = user.channel::<String>("hello");
    // tasks::spawn(async move {
    //     loop {
    //         let message = hello_channel.recv().await.expect("failed to receive");
    //         println!("channel: {message:?}");
    //     }
    // });

    loop {
        match user.channel("hello").send("Hello, world!") {
            Ok(_) => println!("sent message"),
            Err(SendError::Closed) => {
                println!("channel closed");
                break;
            }
            Err(e) => println!("failed to send message: {e}"),
        }
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
