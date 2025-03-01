use maf::{self, App, Body};

fn test_rpc(body: Body<i32>) -> i32 {
    maf::log!("test_rpc: {:?}", body);
    42
}

fn build() -> App {
    loop {
        maf::log!("building app");
    }
    App::new().add_rpc_function("test", test_rpc)
}

maf::register_build!(build);
