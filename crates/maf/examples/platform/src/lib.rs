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
        "count" // This name will be used to identify the store
    }
}

fn increment_counter(
    app: App,
    Params(counter): Params<i32>,
    mut store: StoreMut<CounterStore>,
) -> i32 {
    store.count += counter;

    println!(
        "incremented counter by {counter}. new value: {}",
        store.count
    );

    app.meta()
        .set(MetaVisibility::Public, "count", store.count)
        .expect("failed to set meta");

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
        .select("counter_times_two", |counter: StoreRef<CounterStore>| {
            counter.count * 2
        })
        .meta(
            MetaVisibility::Public,
            "count",
            |counter: StoreRef<CounterStore>| counter.count,
        )
        .meta(MetaVisibility::Public, "users", |users: Users| async move {
            println!("{} connected user(s)", users.count().await);
            users.count().await
        })
        .meta(
            MetaVisibility::Public,
            "count",
            |store: StoreRef<CounterStore>| store.count,
        )
        .meta(MetaVisibility::Public, "greeting", || "hello world!")
        .build()
}

maf::register!(build);
