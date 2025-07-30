mod desc;

pub use desc::StoreDesc;

use crate::App;

pub fn export_types(app: &App) {
    let stores = app
        .inner
        .state
        .stores
        .try_read()
        .expect("Failed to read stores");

    for (key, store) in stores.iter() {
        println!("store: key = {:?}, desc = {:?}", key.as_ref(), store.desc);
    }
}
