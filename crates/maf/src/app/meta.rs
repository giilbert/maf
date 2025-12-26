//! An interface to inactive with the MAF Meta API.

use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};

use crate::platform::{Platform, TargetPlatform};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaVisibility {
    Public,
    Private,
}

/// An entry in the MAF Meta API.
///
/// The value stored in the entry should be unmarshalled from JSON using [`MetaEntry::deserialize`].
#[derive(Debug, Clone)]
pub struct MetaEntry {
    pub(crate) visibility: MetaVisibility,
    pub(crate) value: String,
}

impl MetaEntry {
    /// Deserialize the value of the meta entry into the specified type.
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.value)
    }

    /// Get the visibility of the meta entry.
    pub fn visibility(&self) -> &MetaVisibility {
        &self.visibility
    }
}

///! An interace that holds entries for the MAF Meta API.
pub struct MetaStorage {
    platform: Arc<TargetPlatform>,
}

/// TODO: Error handling with setting entries that are too large. This currently traps in WASI.
#[derive(Debug, thiserror::Error)]
pub enum MetaError {
    #[error("Failed to serialize JSON: {0}")]
    SerializationError(serde_json::Error),
    #[error("Failed to deserialize JSON: {0}")]
    DeserializationError(serde_json::Error),
    #[error("Meta entry not found")]
    NotFound,
}

impl MetaStorage {
    pub fn new(platform: Arc<TargetPlatform>) -> Self {
        Self { platform }
    }

    /// Sets a meta entry with the specified key and value, returning the previous entry if it
    /// existed.
    pub fn set(
        &self,
        visibility: MetaVisibility,
        key: &str,
        value: impl Serialize,
    ) -> Result<Option<MetaEntry>, MetaError> {
        Ok(self.platform.set_meta(
            visibility,
            key,
            &serde_json::to_string(&value).map_err(MetaError::SerializationError)?,
        ))
    }

    /// Gets a meta entry with the specified key, deserialized into the specified type.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<T, MetaError> {
        if let Some(entry) = self.platform.get_meta(key) {
            let value = entry
                .deserialize()
                .map_err(MetaError::DeserializationError)?;
            Ok(value)
        } else {
            Err(MetaError::NotFound)?
        }
    }

    /// Gets a meta entry with the specified key.
    pub fn get_any(&self, key: &str) -> Option<MetaEntry> {
        self.platform.get_meta(key)
    }

    /// Deletes a meta entry with the specified key, returning the previous entry if it existed.
    pub fn delete(&self, key: &str) -> Option<MetaEntry> {
        self.platform.delete_meta(key)
    }

    /// Lists all meta entries.
    pub fn list(&self) -> Vec<(String, MetaEntry)> {
        self.platform.list_meta()
    }
}
