use std::{path::Path, sync::Arc};

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Bundle {
    pub wasm_module: Arc<[u8]>,
}

impl Bundle {
    pub fn load_wasm_module_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let wasm_module = std::fs::read(path).context("failed to read WASM module file")?;
        let wasm_module = Arc::from(wasm_module);
        Ok(Bundle { wasm_module })
    }
}
