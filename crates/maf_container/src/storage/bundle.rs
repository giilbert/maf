use std::{path::Path, sync::Arc};

use async_zip::{error::ZipError, tokio::read::seek::ZipFileReader};
use tokio::{fs::File, io::BufReader};

#[derive(Debug, Clone)]
pub struct BundleStorage {}

#[derive(Debug)]
pub struct Bundle {
    pub wasm_module: Arc<[u8]>,
}

impl BundleStorage {
    pub fn new() -> Self {
        Self {}
    }

    // TODO: load more than just the wasm module
    async fn load_bundle_from_path(&self, path: impl AsRef<Path>) -> anyhow::Result<Bundle> {
        let mut file = BufReader::new(File::open(path).await?);
        let mut zip = ZipFileReader::with_tokio(&mut file).await?;

        for entry_index in 0.. {
            let mut entry_reader = match zip.reader_with_entry(entry_index).await {
                Ok(reader) => reader,
                Err(ZipError::EntryIndexOutOfBounds) => break,
                Err(err) => return Err(err.into()),
            };

            let entry = entry_reader.entry();
            // look for a module.wasm
            if entry.filename().as_str()? == "module.wasm" {
                let mut data = Vec::new();
                entry_reader.read_to_end_checked(&mut data).await?;

                return Ok(Bundle {
                    wasm_module: Arc::from(data),
                });
            }
        }

        anyhow::bail!("no module.wasm found in zip");
    }

    pub async fn load_test_app(&self) -> anyhow::Result<Bundle> {
        const PATH: &'static str = "target/wasm32-wasip2/debug/example_basic.wasm";
        Ok(Bundle {
            wasm_module: Arc::from(tokio::fs::read(PATH).await?),
        })
    }
}
