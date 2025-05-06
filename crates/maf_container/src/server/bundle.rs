use std::sync::Arc;

#[derive(Debug)]
pub struct Bundle {
    pub wasm_module: Arc<[u8]>,
}

impl Bundle {
    pub fn load_wasm_module_from_file(path: &str) -> anyhow::Result<Self> {
        let wasm_module = std::fs::read(path)?;
        let wasm_module = Arc::from(wasm_module);
        Ok(Bundle { wasm_module })
    }
}
