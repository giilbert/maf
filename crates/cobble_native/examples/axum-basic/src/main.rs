use std::sync::Arc;

use cobble_native::{
    axum::{CobbleRoutes, Room, RouteConfig},
    prelude::*,
};
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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

async fn increment_counter(Params(counter): Params<i32>, store: Store<CounterStore>) -> i32 {
    let mut store = store.write().await;

    store.count += counter;

    tracing::info!(
        target: "cobble_app",
        "incremented counter by {counter}. new value: {}",
        store.count
    );

    store.count
}

fn on_connect(user: User) {
    tracing::info!(
        target: "cobble_app",
        "user connected! id: {}",
        user.meta.id()
    );
}

fn app() -> AppBuilder {
    App::builder()
        .on_connect(on_connect)
        .store::<CounterStore>()
        .rpc("increment_counter", increment_counter)
        .select("count_times_two", |store: Store<CounterStore>| async move {
            store.read().await.count * 2
        })
        .background(|_app: App| async move {
            tracing::info!(target: "cobble_app", "hello world!");
        })
}

#[derive(Clone)]
pub struct AppState {
    room: Arc<RwLock<Room>>,
}

impl RouteConfig for AppState {
    type GetRoomParams = ();
    async fn get_room(&self, params: Self::GetRoomParams) -> Option<Room> {
        tracing::info!("getting room with params: {params:?}");
        Some(self.room.read().await.clone())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let room = Room::new(app())?;

    let routes = CobbleRoutes::<AppState>::new();
    let router = axum::Router::<AppState>::new()
        .nest("/@/_/_/default", routes.connect_handler())
        .with_state::<()>(AppState {
            room: Arc::new(RwLock::new(room)),
        });

    let address = "0.0.0.0:1147";
    tracing::info!("Server listening on {address}");

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
