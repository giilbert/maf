use maf::*;
use schemars::JsonSchema;

#[derive(Debug, Clone, JsonSchema, serde::Serialize)]
struct LightsOut {
    /// Indices represent the tiles in a 8x8 grid:
    ///  0  1  2  3  4  5  6  7
    ///  8  9 10 11 12 13 14 15
    /// 16 17 18 19 20 21 22 23
    /// 24 25 26 27 28 29 30 31
    /// 32 33 34 35 36 37 38 39
    /// 40 41 42 43 44 45 46 47
    /// 48 49 50 51 52 53 54 55
    /// 56 57 58 59 60 61 62 63
    tiles: Vec<bool>,
    people: u32,
}

impl StoreData for LightsOut {
    type Select<'this> = &'this LightsOut;

    fn init() -> Self {
        LightsOut {
            tiles: vec![false; 64],
            people: 0,
        }
    }

    fn name() -> impl AsRef<str> + Send {
        "LightsOut"
    }

    fn select(&self, _user: &User) -> Self::Select<'_> {
        self
    }
}

async fn toggle_tile(data: Store<LightsOut>, Params(index): Params<usize>) {
    if index >= 64 {
        return; // Index out of bounds
    }

    let mut data = data.write().await;

    data.tiles[index] = !data.tiles[index];

    if index >= 8 {
        // Not on top edge, toggle the tile above
        data.tiles[index - 8] = !data.tiles[index - 8];
    }

    if index < 56 {
        // Not on bottom edge, toggle the tile below
        data.tiles[index + 8] = !data.tiles[index + 8];
    }

    if index % 8 != 7 {
        // Not on right edge, toggle the tile to the right
        data.tiles[index + 1] = !data.tiles[index + 1];
    }

    if index % 8 != 0 {
        // Not on left edge, toggle the tile to the left
        data.tiles[index - 1] = !data.tiles[index - 1];
    }
}

async fn on_connect(data: Store<LightsOut>) {
    data.write().await.people += 1;
}

async fn on_disconnect(data: Store<LightsOut>) {
    data.write().await.people -= 1;
}

fn build() -> App {
    App::builder()
        .store::<LightsOut>()
        .rpc("toggle_tile", toggle_tile)
        .rpc("greet", |Params(name): Params<String>| {
            format!("Hello, {}!", name)
        })
        .on_connect(on_connect)
        .on_disconnect(on_disconnect)
        .build()
}

// maf::register!(build);
