use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use async_zip::{error::ZipError, tokio::read::seek::ZipFileReader};
use bytes::Bytes;
use futures_util::{AsyncReadExt, Stream, StreamExt, TryStreamExt};
use cobble_container::server::Bundle;
use cobble_schemas::error::ErrorResponse;
use tokio::{
    fs::{self, File},
    io::{AsyncBufRead, AsyncSeek, BufReader},
    sync::mpsc,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BundleStorage {
    pub storage_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("Bundle file not found")]
    FileNotFound,
    #[error("File too large")]
    FileTooLarge,
    #[error("Invalid zip file")]
    InvalidZip,
    #[error("Entry reader error")]
    EntryReader(#[from] async_zip::error::ZipError),
    #[error("IO error: {0}")]
    Io(#[from] tokio::io::Error),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl BundleStorage {
    pub async fn new() -> anyhow::Result<Self> {
        let storage_dir = dotenvy::var("BUNDLE_STORAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("var/bundles"));

        if !tokio::fs::try_exists(&storage_dir).await? {
            tokio::fs::create_dir_all(&storage_dir).await?;
            tracing::info!(
                "Created bundle storage directory: {}",
                storage_dir.display()
            );
        } else {
            tracing::info!(
                "Bundle storage directory already exists: {}",
                storage_dir.display()
            );
        }

        Ok(Self { storage_dir })
    }

    pub async fn upload_bundle(
        &self,
        app_id: Uuid,
        stream: impl Stream<Item = Result<Bytes, axum::Error>>,
    ) -> Result<(), BundleError> {
        tokio::pin!(stream);

        let path = self.storage_dir.join(app_id.to_string());
        let mut file = fs::File::create(&path).await?;

        const MAX_SIZE: usize = 20 * 1024 * 1024; // 20 MB
        let mut size = 0;

        let (abort_tx, mut abort_rx) = mpsc::channel::<()>(1);

        let mut reader = tokio_util::io::StreamReader::new(
            stream
                .map(|item| item.map_err(|_| BundleError::InvalidZip))
                .inspect_ok(|item| {
                    size += item.len();
                    if size > MAX_SIZE {
                        abort_tx.try_send(()).ok();
                    }
                }),
        );

        tokio::select! {
            result = tokio::io::copy(&mut reader, &mut file) => {
                result?;
            },
            _ = abort_rx.recv() => {
                return Err(BundleError::FileTooLarge);
            }
        }

        // Reopen the file to read from the beginning
        file = fs::File::open(path).await?;

        match self
            .load_bundle_from_reader(BufReader::new(file), true)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(BundleError::InvalidZip).context(e)?,
        }
    }

    async fn load_bundle_from_reader(
        &self,
        mut reader: impl AsyncBufRead + AsyncSeek + Unpin,
        ignore_data: bool,
    ) -> Result<Option<Bundle>, BundleError> {
        let mut zip = ZipFileReader::with_tokio(&mut reader).await?;

        for entry_index in 0.. {
            let mut entry_reader = match zip.reader_with_entry(entry_index).await {
                Ok(reader) => reader,
                Err(ZipError::EntryIndexOutOfBounds) => break,
                Err(err) => return Err(err.into()),
            };

            let entry = entry_reader.entry();

            tracing::debug!(
                "Found entry: {} (size: {})",
                entry.filename().as_str()?,
                entry.compressed_size()
            );

            // look for a module.wasm
            if entry.filename().as_str()? == "module.wasm" {
                const WASM_MODULE_MAX_SIZE: usize = 20 * 1024 * 1024; // 20 MB

                let size = entry.uncompressed_size() as usize;
                if size > WASM_MODULE_MAX_SIZE {
                    return Err(BundleError::FileTooLarge);
                }

                if ignore_data {
                    return Ok(None);
                }

                let mut data = vec![0; size];

                entry_reader
                    .read_exact(&mut data)
                    .await
                    .map_err(|e| BundleError::Io(e))?;

                return Ok(Some(Bundle {
                    wasm_module: Arc::from(data),
                }));
            }
        }

        return Err(BundleError::InvalidZip);
    }

    pub async fn load_app_bundle(&self, app_id: Uuid) -> Result<Option<Bundle>, BundleError> {
        Ok(
            match self
                .load_bundle_from_path(self.storage_dir.join(app_id.to_string()))
                .await
            {
                Ok(bundle) => Some(bundle),
                Err(BundleError::FileNotFound) => None,
                Err(e) => {
                    return Err(e);
                }
            },
        )
    }

    // TODO: load more than just the wasm module
    async fn load_bundle_from_path(&self, path: impl AsRef<Path>) -> Result<Bundle, BundleError> {
        let mut file = BufReader::new(File::open(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BundleError::FileNotFound
            } else {
                BundleError::Io(e)
            }
        })?);

        self.load_bundle_from_reader(&mut file, false)
            .await
            .map(|bundle| bundle.expect("data should be present"))
    }

    pub async fn load_test_app(&self) -> anyhow::Result<Bundle> {
        const PATH: &'static str = "target/wasm32-wasip2/debug/example_basic.wasm";
        Ok(Bundle {
            wasm_module: Arc::from(tokio::fs::read(PATH).await?),
        })
    }

    pub async fn delete_app_bundle(&self, app_id: Uuid) -> Result<(), BundleError> {
        let path = self.storage_dir.join(app_id.to_string());

        fs::rename(&path, path.with_extension("deleted"))
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BundleError::FileNotFound
                } else {
                    BundleError::Io(e)
                }
            })?;

        fs::remove_file(path.with_extension("deleted"))
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BundleError::FileNotFound
                } else {
                    BundleError::Io(e)
                }
            })?;

        Ok(())
    }
}

impl Into<std::io::Error> for BundleError {
    fn into(self) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, self)
    }
}

impl BundleError {
    pub fn error_response(self) -> ErrorResponse {
        match self {
            BundleError::FileTooLarge => ErrorResponse::bad_request(Some("File too large")),
            BundleError::InvalidZip => ErrorResponse::bad_request(Some("Invalid zip file")),
            other => ErrorResponse::from(other),
        }
    }
}
