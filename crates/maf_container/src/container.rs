#[derive(Debug)]
pub struct Container {
    path: String,
    module: wasmtime::Module,
    instance: wasmtime::Instance,
    store: wasmtime::Store<ContainerData>,
}

#[derive(Debug, Default)]
pub struct ContainerData {}

impl Container {
    pub fn load_from_file(
        engine: &wasmtime::Engine,
        linker: &wasmtime::Linker<ContainerData>,
        path: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;

        let module = wasmtime::Module::new(engine, &bytes)?;

        let mut store = wasmtime::Store::new(engine, ContainerData::default());
        let instance = linker.instantiate(&mut store, &module)?;

        Ok(Self {
            path: path.to_string(),
            module,
            instance,
            store,
        })
    }
}
