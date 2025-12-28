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

// RPC functions can be used to perform actions on the server
fn increment_counter(
    // Special types for extracting parameters, data, and context
    Params(inc): Params<i32>,
    mut counter: StoreMut<CounterStore>,
) -> i32 {
    counter.count += inc;
    println!("incremented counter by {inc}. new value: {}", counter.count);
    counter.count
}

fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta().id());
}

// Declare what the MAF application should do
fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .rpc("increment_counter", increment_counter)
        .build()
}

maf::register!(build);
