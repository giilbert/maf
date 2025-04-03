pub mod bindgen {
    wit_bindgen::generate!({
        path: "../../wit",
        pub_export_macro: true,
        with: {
            "wasi:io/poll@0.2.4": wasi::io::poll
        }
    });

    pub use maf::bindings::bindings::*;
}

pub fn init_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| println!("{}", panic_info)));
}
