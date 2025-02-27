mod container;
mod runtime;

use wasmtime::*;

fn main() -> anyhow::Result<()> {
    let wasm_module_path = std::env::args()
        .nth(1)
        .expect("missing wasm module path in first argument");

    let engine = Engine::new(&Config::new().wasm_memory64(false))?;

    let bytes = std::fs::read(wasm_module_path)?;
    let module = Module::new(&engine, bytes)?;

    let mut linker = Linker::new(&engine);

    linker.func_wrap(
        "maf",
        "ffi_print",
        |mut caller: Caller<'_, ()>, ptr: u32, len: u64| -> anyhow::Result<()> {
            let memory = caller
                .get_export("memory")
                .ok_or_else(|| anyhow::anyhow!("failed to find `memory` export on caller"))?
                .into_memory()
                .ok_or_else(|| anyhow::anyhow!("`memory` export on caller is not a memory"))?;

            let data = memory.data(&caller).get(ptr as usize..).ok_or_else(|| {
                anyhow::anyhow!("failed to get data from memory at offset {}", ptr)
            })?;

            let data = &data[..len as usize];

            let message = std::str::from_utf8(data)?;

            println!("{}", message);
            // println!("memory size = {}", memory.size(&caller));

            Ok(())
        },
    )?;

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;

    let init = instance.get_typed_func::<(), ()>(&mut store, "init")?;
    init.call(&mut store, ())?;

    let alloc = instance.get_typed_func::<(u32, u32), u32>(&mut store, "alloc")?;

    let ptr = alloc.call(&mut store, (12, 1))?;
    println!("Allocated memory at: {}", ptr);

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow::anyhow!("failed to find `memory` export on instance"))?;
    memory.write(&mut store, ptr as usize, b"Hello, Wasm!")?;

    let handle_request = instance.get_typed_func::<(u32, u64), ()>(&mut store, "handle_request")?;
    handle_request.call(&mut store, (ptr, 12))?;

    Ok(())
}
