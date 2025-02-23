use maf;
use std::io::Write as _;

#[link(wasm_import_module = "maf")]
extern "C" {
    fn ffi_print(ptr: *const u8, len: u64);
}

fn print(message: &str) {
    unsafe {
        ffi_print(message.as_ptr(), message.len() as u64);
    }
}

#[no_mangle]
pub extern "C" fn init() {
    std::panic::set_hook(Box::new(|panic_info| {
        let message = panic_info.to_string();
        print(&message);
    }));
    // std::fs::read("maf.wasm").unwrap();
    // print("Hello, World!\n");
    println!("Hello, World!");
}

#[no_mangle]
pub extern "C" fn handle_request(path_buffer: *mut u8, path_length: usize) {
    let path = unsafe { String::from_raw_parts(path_buffer, path_length, path_length) };
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
