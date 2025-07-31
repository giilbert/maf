mod desc;

pub use desc::{ExtractRpcDesc, RpcDesc, StoreDesc};
use facet::Facet;

use crate::App;

#[derive(facet::Facet)]
#[repr(C)]
pub enum TestEnum {
    Variant1 { field1: i32 },
    Variant2 { field1: String },
}

pub fn export_types(app: &App) {
    let stores = app
        .inner
        .state
        .stores
        .try_read()
        .expect("Failed to read stores");
    let rpcs = &app.inner.rpc_functions.inner;

    println!("{:?}", TestEnum::SHAPE);

    for (key, store) in stores.iter() {
        println!("store: key = {:?}, desc = {:?}", key.as_ref(), store.desc);
    }

    for rpc in rpcs.values() {
        println!("rpc: method = {:?} desc = {:?}", rpc.method, rpc.desc);
    }
}
