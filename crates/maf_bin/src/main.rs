use wasmtime::*;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let wasm_module_path = std::env::args()
        .nth(1)
        .expect("missing wasm module path in first argument");

    let bytes = std::fs::read(wasm_module_path)?;

    let engine = Engine::default();
    let module = Module::new(&engine, bytes)
        .map_err(|e| eyre::eyre!("failed to instantiate webassembly module {e:?}"))?;

    let mut linker = Linker::new(&engine);

    linker
        .func_wrap("maf", "foo", |caller: Caller<'_, ()>, x: u64| {
            println!("foo({})", x);
        })
        .map_err(|e| eyre::eyre!("failed to wrap foo: {e:?}"))?;

    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| eyre::eyre!("failed to instantiate module: {e:?}"))?;
    let init = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .map_err(|e| eyre::eyre!("failed to instantiate module: {e:?}"))?;

    init.call(&mut store, ())
        .map_err(|e| eyre::eyre!("failed to call init: {e:?}"))?;

    Ok(())
}
