pub mod bindgen {
    wit_bindgen::generate!({
        path: "../../crates/maf_container/src/runtime/wasi",
        pub_export_macro: true,
    });
}

pub fn print_str(message: impl AsRef<str>) {
    bindgen::print(message.as_ref()).expect("failed to print");
}

#[macro_export]
macro_rules! log {
    () => {
        $crate::bindings::print_str("");
    };
    ($($arg:tt)*) => {{
        $crate::bindings::print_str(&format!($($arg)*));
    }};
}

pub fn init_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| log!("{}", panic_info)));
}
