use maf::*;

fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta.id());
}

fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .background(|app: App| async move {
            println!("hello world!");

            let res = http::Request::get("https://www.google.com")
                .send()
                .await
                .expect("request failed");
        })
        .build()
}

maf::register!(build);
