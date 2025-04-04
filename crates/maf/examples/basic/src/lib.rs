use maf::{self, serde_json::json, tasks, App, Body, UserListener};

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

fn build() -> App {
    let app = App::new().add_rpc_function("test", test_rpc);

    tasks::spawn(async move {
        let users = UserListener::new().expect("failed to create user listener");

        loop {
            let result = users.next().await;

            match result {
                Ok(user) => {
                    println!("got user");
                    user.send(json!({
                        "message": "hello world!"
                    }))
                    .expect("failed to send data");
                }
                Err(err) => {
                    println!("error: {:?}", err);
                    break;
                }
            }
        }
    });

    app
}

maf::register_build!(build);
