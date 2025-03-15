use maf::{self, tasks, App, Body};

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

fn build() -> App {
    let app = App::new().add_rpc_function("test", test_rpc);

    let runtime = tasks::Runtime::new();
    let capture = 2;

    runtime
        .spawn(async move {
            println!("Hello from async task! capture = {capture}");
        })
        .on_finish(|_| {
            println!("Task finished!");
        });

    app
}

maf::register_build!(build);
