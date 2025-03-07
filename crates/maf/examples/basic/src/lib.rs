use maf::{self, tasks, App, Body};

fn test_rpc(body: Body<i32>) -> i32 {
    maf::log!("test_rpc: {:?}", body);
    42
}

fn build() -> App {
    let app = App::new().add_rpc_function("test", test_rpc);

    let runtime = tasks::Runtime::new();
    let capture = 2;

    runtime
        .spawn(async move {
            maf::log!("Hello from async task! capture = {capture}");
        })
        .on_finish(|_| {
            maf::log!("Task finished!");
        });

    app
}

maf::register_build!(build);
