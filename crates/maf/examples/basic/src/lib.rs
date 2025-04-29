use std::time::Duration;

use maf::*;

fn test_rpc(body: Params<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

async fn on_connect(user: User) {
    println!("user connected!");

    loop {
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
