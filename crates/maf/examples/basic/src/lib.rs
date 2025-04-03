use maf::{self, tasks, wasi, App, Body};

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

fn build() -> App {
    let app = App::new().add_rpc_function("test", test_rpc);

    tasks::spawn(async move {
        println!("Hello from async task!");
        let now = wasi::clocks::monotonic_clock::now();
        println!("Monotonic clock: {:?}", now);

        let deadline = now + 1_000_000_000;
        tasks::sleep_until(deadline).await;

        println!("Task resumed after sleep!");
    })
    .on_finish(|_| {
        println!("Task finished!");
    });

    app
}

maf::register_build!(build);
