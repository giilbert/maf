#[link(wasm_import_module = "maf")]
extern "C" {
    // Expects a pointer to the start of a Rust UTF-8 &str and its length.
    fn ffi_print(str_pointer: *const u8, str_length: u64);
}

pub fn print_str(message: impl AsRef<str>) {
    let message = message.as_ref();
    // SAFETY: The pointer and length are valid UTF-8 checked by Rust.
    unsafe {
        ffi_print(message.as_ptr(), message.len() as u64);
    }
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

#[no_mangle]
pub extern "C" fn handle_request(path_buffer: *mut u8, path_length: u64) {
    log!("{path_buffer:?}, {path_length}");
    let path =
        unsafe { String::from_raw_parts(path_buffer, path_length as usize, path_length as usize) };
    log!("Handling request for path: {}", path);
}

/// Hook for the allocator to be used by the WebAssembly engine. This function returns a pointer to
/// the allocated memory block in WASM memory. If memory cannot be allocated (i.e. in case of an
/// OOM error), this function will return `0`.
///
/// The engine should ensure that the memory allocated by this function is deallocated by the
/// program.
#[no_mangle]
pub extern "C" fn alloc(size: usize, align: usize) -> *mut u8 {
    assert!(size > 0, "size must be greater than 0");

    unsafe {
        // SAFETY: `size` and `align` are validated.
        std::alloc::alloc(
            std::alloc::Layout::from_size_align(size, align).expect("invalid size or alignment"),
        )
    }
}

pub fn init_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| log!("{}", panic_info)));
}
