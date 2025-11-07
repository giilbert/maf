pub mod bindgen {
    wit_bindgen::generate!({
        path: "wit",
        pub_export_macro: true,
        with: {
            "wasi:io/poll@0.2.6": wasi::io::poll
        }
    });

    #[allow(unused)] // This is used in generated code
    pub use maf::bindings::bindings::*;
}

#[allow(unused)] // This is used in generated code
pub fn init_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| println!("{}", panic_info)));
}
