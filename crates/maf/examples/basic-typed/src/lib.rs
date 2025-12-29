use maf::prelude::*;

struct CounterStore {
    count: i32,
}

impl StoreData for CounterStore {
    type Select<'this> = Option<i32>;

    fn init() -> Self {
        CounterStore { count: 0 }
    }

    fn select(&self, _user: &User) -> Self::Select<'_> {
        Some(self.count)
    }

    fn name() -> impl AsRef<str> {
        "count" // This name will be used to identify the store
    }
}

fn increment_counter(Params(inc): Params<i32>, mut counter: StoreMut<CounterStore>) -> i32 {
    counter.count += inc;

    println!("incremented counter by {inc}. new value: {}", counter.count);

    counter.count
}

fn counter_read_hook(counter: StoreRef<CounterStore>) -> i32 {
    println!("counter read hook: {}", counter.count);
    counter.count
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
