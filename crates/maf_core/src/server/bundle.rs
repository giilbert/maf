use std::sync::Arc;

use maf_schemas::project_config::ProjectConfigFile;

/// A bundle is a full package of the resources and configuration needed to run a MAF room.
#[derive(Debug, Clone)]
pub struct Bundle {
    /// Loaded from `maf-project.toml`.
    config: Arc<ProjectConfigFile>,
    /// The bytes of the WASM module that will be loaded into the room's container.
    wasm: Arc<[u8]>,
}

impl Bundle {
    pub fn from_wasm_bytes(config: ProjectConfigFile, bytes: Arc<[u8]>) -> anyhow::Result<Self> {
        Ok(Bundle {
            config: Arc::new(config),
            wasm: bytes,
        })
    }

    pub fn wasm_module_bytes(&self) -> &[u8] {
        &self.wasm
    }

    pub fn config(&self) -> &ProjectConfigFile {
        &self.config
    }
}
