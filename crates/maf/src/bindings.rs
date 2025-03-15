pub mod bindgen {
    wit_bindgen::generate!({
        path: "../../crates/maf_container/src/runtime/wasi",
        pub_export_macro: true,
    });
}

//     std::panic::set_hook(Box::new(|panic_info| print!("{}", panic_info)));
// }
