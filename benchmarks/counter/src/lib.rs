use maf::prelude::*;

struct CounterStore {
    count: i32,
}

impl StoreData for CounterStore {
    type Select<'this> = i32;

    fn init() -> Self {
        CounterStore { count: 0 }
    }

    fn select(&self, _user: &User) -> Self::Select<'_> {
        self.count
    }

    fn name() -> impl AsRef<str> {
        "counter"
    }
}

// RPC functions can be used to perform actions on the server
fn increment_counter(
    // Special types for extracting parameters, data, and context
    Params(inc): Params<i32>,
    // test: Store<CounterStore>,
    mut counter: StoreMut<CounterStore>,
) -> i32 {
    counter.count += inc;
    counter.count
}

fn noop() {
    println!("noop called");
}

async fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta().id());
}

fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .store::<CounterStore>()
        .rpc("increment_counter", increment_counter)
        .rpc("noop", noop)
        .build()
}

maf::register!(build);
