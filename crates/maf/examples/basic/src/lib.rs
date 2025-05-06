use maf::*;

struct CounterStore;

impl StoreData for CounterStore {
    type Data = i32;

    fn init() -> Self::Data {
        42
    }
}

async fn increment_counter(Params(counter): Params<i32>, test: Store<CounterStore>) -> i32 {
    let mut data = test.write().await;

    *data += counter;

    println!("incremented counter by {counter}. new value: {}", &*data);

    *data
}

async fn on_connect(user: User) {
    println!("!!! user connected! id: {}", user.meta.id());
    println!("HAIII");
}

fn build() -> App {
    App::builder()
        .on_connect(on_connect)
        .rpc("increment_counter", increment_counter)
        .background(|app: App| async move {
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
