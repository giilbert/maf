use std::time::Duration;

use maf::*;

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

async fn on_connect(user: User) {
    println!("user connected!");

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
        .background(|app: App| async move {
            let mut chan = app.channel::<String>("hello");
            loop {
                match chan.recv().await {
                    Ok(msg) => println!("got message! `{msg}`"),
                    Err(e) => println!("failed to receive message: {e}"),
                }
            }
        })
        .build()
}

maf::register!(build);
