use maf::*;

fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta.id());
}

fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .background(|app: App| async move {
            println!("hello world!");

            let res = http::get("https://www.google.com")
                .send()
                .await
                .expect("request failed")
                .text()
                .await
                .expect("failed to read response text");

            println!("Response: {}", res);
        })
        .build()
}

maf::register!(build);
