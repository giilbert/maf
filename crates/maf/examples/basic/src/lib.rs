use std::sync::atomic::AtomicUsize;

use maf::{self, App, Body};

fn test_rpc(body: Body<i32>) -> i32 {
    maf::log!("test_rpc: {:?}", body);
    42
}

static NUMBER: AtomicUsize = AtomicUsize::new(0);
fn build() -> App {
    loop {
        maf::log!(
            "building app {}",
            NUMBER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
    }
    App::new().add_rpc_function("test", test_rpc)
}

maf::register_build!(build);
