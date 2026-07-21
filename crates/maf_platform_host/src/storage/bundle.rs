use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use async_zip::error::ZipError;
use async_zip::tokio::read::seek::ZipFileReader;
use bytes::Bytes;
use futures_util::{AsyncReadExt, Stream, StreamExt, TryStreamExt};
use maf_core::server::Bundle;
use maf_schemas::error::ErrorResponse;
use maf_schemas::project_config::ProjectConfigFile;
use tokio::fs::{self, File};
use tokio::io::{AsyncBufRead, AsyncSeek, BufReader};
use tokio::sync::mpsc;
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
        app_config: ProjectConfigFile,
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

        // Validate that the uploaded file is a valid zip and contains the expected contents,
        // without reading the potentially large WASM module data.
        match self
            .load_bundle_from_zip_reader(
                app_config,
                BufReader::new(file),
                // Validate that the WASM file is present without reading the potentially large WASM
                // module data.
                false,
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(BundleError::InvalidZip).context(e)?,
        }
    }

    /// Loads a bundle from a zip file. If `should_return_wasm` is true, the zip file will be parsed
    /// and the WASM module data will be validated and returned. This can be used to validate the
    /// zip file and extract metadata without reading the potentially large WASM module data.
    ///
    /// Returns:
    /// - `Ok(None)` if the zip file is valid and contains the expected contents, but the WASM
    ///   module data is not returned (`should_return_wasm` is false).
    /// - `Ok(Some(Bundle))` if the zip file is valid and contains the expected contents, and the
    ///   WASM module data is returned (`should_return_wasm` is true).
    /// - `Err(BundleError)` if the zip file is invalid or does not contain the expected contents.
    async fn load_bundle_from_zip_reader(
        &self,
        app_config: ProjectConfigFile,
        mut zip_reader: impl AsyncBufRead + AsyncSeek + Unpin,
        should_return_wasm: bool,
    ) -> Result<Option<Bundle>, BundleError> {
        let mut zip = ZipFileReader::with_tokio(&mut zip_reader).await?;

        for entry_index in 0.. {
            let mut entry_reader = match zip.reader_with_entry(entry_index).await {
                Ok(reader) => reader,
                Err(ZipError::EntryIndexOutOfBounds) => break,
                Err(err) => return Err(err.into()),
            };

            let entry = entry_reader.entry();

            tracing::debug!(
                "found entry: {} (size: {})",
                entry.filename().as_str()?,
                entry.compressed_size()
            );

            // TODO: load more than just the wasm module
            // Look for a module.wasm file: this is the expected name for the WASM module in the
            // bundle
            if entry.filename().as_str()? == "module.wasm" {
                const WASM_MODULE_MAX_SIZE: usize = 20 * 1024 * 1024; // 20 MB

                let size = entry.uncompressed_size() as usize;
                if size > WASM_MODULE_MAX_SIZE {
                    return Err(BundleError::FileTooLarge);
                }

                if !should_return_wasm {
                    return Ok(None);
                }

                let mut data = vec![0; size];

                entry_reader
                    .read_exact(&mut data)
                    .await
                    .map_err(BundleError::Io)?;

                return Ok(Some(Bundle::from_wasm_bytes(app_config, Arc::from(data))?));
            }
        }

        Err(BundleError::InvalidZip)
    }

    /// Loads the bundle for the given app ID, if it exists.
    ///
    /// TODO: this error handing is weird--there is no way to distinguish between an invalid zip
    /// file or the app not existing
    pub async fn load_app_bundle(
        &self,
        app_config: ProjectConfigFile,
        app_id: Uuid,
    ) -> Result<Option<Bundle>, BundleError> {
        Ok(
            match self
                .load_bundle_from_path(app_config, self.storage_dir.join(app_id.to_string()))
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

    async fn load_bundle_from_path(
        &self,
        app_config: ProjectConfigFile,
        path: impl AsRef<Path>,
    ) -> Result<Bundle, BundleError> {
        let mut file = BufReader::new(File::open(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BundleError::FileNotFound
            } else {
                BundleError::Io(e)
            }
        })?);

        self.load_bundle_from_zip_reader(app_config, &mut file, true)
            .await
            .map(|bundle| bundle.ok_or(BundleError::FileNotFound))?
    }

    pub async fn load_test_app(&self) -> anyhow::Result<Bundle> {
        // const PATH: &str = "target/wasm32-wasip2/debug/example_basic.wasm";
        // let bytes = Arc::from(tokio::fs::read(PATH).await?),
        // Ok(Bundle)
        todo!();
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

impl From<BundleError> for std::io::Error {
    fn from(val: BundleError) -> Self {
        std::io::Error::other(val)
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
