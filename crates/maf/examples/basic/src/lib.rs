use maf;

#[link(wasm_import_module = "maf")]
extern "C" {
    fn foo(x: u64);
}

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        foo(maf::add(41, 1));
    }
}
