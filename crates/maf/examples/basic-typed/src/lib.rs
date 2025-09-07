use maf::prelude::*;

struct CounterStore {
    count: i32,
}

impl StoreData for CounterStore {
    type Select<'a> = Option<&'a i32>;

    fn init() -> Self {
        CounterStore { count: 0 }
    }

    fn select(&self, user: &User) -> Self::Select<'_> {
        Some(&self.count)
    }

    fn name() -> impl AsRef<str> {
        "count" // This name will be used to identify the store
    }
}

async fn increment_counter(Params(counter): Params<i32>, test: Store<CounterStore>, _: u32) -> i32 {
    let mut store = test.write().await;

    store.count += counter;

    println!(
        "incremented counter by {counter}. new value: {}",
        store.count
    );

    store.count
}

async fn counter_read_hook(test: Store<CounterStore>) -> i32 {
    let store = test.read().await;
    println!("counter read hook: {}", store.count);
    store.count
}

fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta().id());
}

fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .store::<CounterStore>()
        .rpc("increment_counter", increment_counter)
        .hook("counter", counter_read_hook)
        .background(|app: App| async move {
            println!("hello world!");
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
