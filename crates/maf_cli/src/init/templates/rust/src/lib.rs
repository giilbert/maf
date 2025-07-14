use maf::*;

struct CounterStore;

impl StoreData for CounterStore {
    type Data = i32;

    fn init() -> Self::Data {
        0
    }

    // Determines what data to send to the client when the store is serialized
    fn select(data: &Self::Data, _user: &User) -> impl serde::Serialize {
        data
    }

    // This name will be used to identify the store
    fn name() -> impl AsRef<str> {
        "count"
    }
}

// RPC functions can be used to perform actions on the server
async fn increment_counter(
    // Special types for extracting parameters, data, and context
    Params(counter): Params<i32>,
    test: Store<CounterStore>,
) -> i32 {
    let mut data = test.write().await;
    *data += counter;
    println!("incremented counter by {counter}. new value: {}", &*data);
    *data
}

async fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta.id());
}

// Declare what the MAF application should do
fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .rpc("increment_counter", increment_counter)
        .build()
}

maf::register!(build);
