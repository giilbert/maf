use maf::*;

struct CounterStore;

impl StoreData for CounterStore {
    type Data = i32;

    fn init() -> Self::Data {
        42
    }

    fn select(data: &Self::Data, _user: &User) -> impl serde::Serialize {
        data
    }
}

async fn increment_counter(Params(counter): Params<i32>, test: Store<CounterStore>) -> i32 {
    let mut data = test.write().await;

    *data += counter;

    println!("incremented counter by {counter}. new value: {}", &*data);

    *data
}

async fn counter_read_hook(test: Store<CounterStore>) -> i32 {
    let data = test.read().await;
    println!("counter read hook: {}", &*data);
    *data
}

fn on_connect(user: User) {
    println!("user connected! id: {}", user.meta.id());
}

fn build() -> App {
    App::builder()
        .on_connect(on_connect)
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
