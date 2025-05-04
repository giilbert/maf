use std::sync::Arc;

#[derive(Debug)]
pub struct Bundle {
    pub wasm_module: Arc<[u8]>,
}
