use std::path::Path;
use std::sync::Arc;

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Bundle {
    pub wasm_module_bytes: Arc<[u8]>,
}

impl Bundle {
    pub fn load_wasm_module_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .with_context(|| format!("failed to read wasm module from file {}", path.display()))?;
        Ok(Bundle {
            wasm_module_bytes: Arc::from(bytes),
        })
    }
}
