use maf::{self, tasks, wasi, App, Body};

fn test_rpc(body: Body<i32>) -> i32 {
    println!("test_rpc: {:?}", body);
    42
}

fn build() -> App {
    let app = App::new().add_rpc_function("test", test_rpc);

    let runtime = tasks::Runtime::new();
    let capture = 2;

    let runtime_clone = runtime.clone();
    runtime
        .spawn(async move {
            println!("Hello from async task! capture = {capture}");
            let now = wasi::clocks::monotonic_clock::now();
            println!("Monotonic clock: {:?}", now);

            let deadline = now + 1_000_000_000;
            let sleep_future = tasks::timers::SleepFuture::new(runtime_clone.clone(), deadline);
            sleep_future.await;

            println!("Task resumed after sleep!");
        })
        .on_finish(|_| {
            println!("Task finished!");
        });

    runtime.blocking_poll();

    app
}

maf::register_build!(build);
