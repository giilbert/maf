use maf::prelude::*;

struct CounterStore {
    count: i32,
}

impl StoreData for CounterStore {
    type Select<'this> = &'this i32;

    fn init() -> Self {
        CounterStore { count: 0 }
    }

    fn select(&self, _user: &User) -> Self::Select<'_> {
        &self.count
    }

    fn name() -> impl AsRef<str> {
        "count" // This name will be used to identify the store
    }
}

fn increment_counter(Params(counter): Params<i32>, mut store: StoreMut<CounterStore>) -> i32 {
    store.count += counter;

    println!(
        "incremented counter by {counter}. new value: {}",
        store.count
    );

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
        .build()
}

maf::register!(build);
