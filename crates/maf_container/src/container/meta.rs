//! The `Meta` feature allows MAF apps to store and retrieve custom metadata that can be accessed
//! with HTTP APIs (e.g. on MAF Platform).
//!
//! This module defines the storage and limits for metadata associated with a [`ContainerData`].

use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::wasi::bindings;

#[derive(Debug, Clone)]
pub struct MetaStorage {
    inner: Arc<MetaStorageInner>,
}

#[derive(Debug)]
struct MetaStorageInner {
    data: RwLock<HashMap<String, MetaEntry<serde_json::Value>>>,
    max_num_keys: usize,
    max_key_size: usize,
    max_value_size: usize,
}

#[derive(Debug, Clone)]
pub struct MetaEntry<T> {
    pub visibility: MetaVisibility,
    pub value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaVisibility {
    /// Public metadata can be accessed by anyone, including clients.
    Public,
    /// Private metadata can only be accessed by the app (running on MAF Platform and service
    /// accounts) itself. Private metadata also includes all public metadata.
    Private,
}

impl MetaVisibility {
    /// Check if `self` can access metadata with the given `visibility`.
    pub fn can_access(self, visibility: MetaVisibility) -> bool {
        match (self, visibility) {
            (MetaVisibility::Private, _) => true,
            (MetaVisibility::Public, MetaVisibility::Public) => true,
            (MetaVisibility::Public, MetaVisibility::Private) => false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetaStorageError {
    #[error("metadata size limit exceeded")]
    SizeLimitExceeded,
    #[error("unable to serialize or deserialize json value: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl MetaStorage {
    pub fn new() -> Self {
        const MAX_KEY_SIZE: usize = 1024; // 1 KB
        const MAX_META_VALUE_SIZE: usize = 8 * 1024; // 8 KB
        const MAX_NUM_KEYS: usize = 64; // 64 keys

        Self {
            inner: Arc::new(MetaStorageInner {
                data: RwLock::new(HashMap::new()),
                max_key_size: MAX_KEY_SIZE,
                max_num_keys: MAX_NUM_KEYS,
                max_value_size: MAX_META_VALUE_SIZE,
            }),
        }
    }

    /// Set a metadata key to a given value.
    pub async fn set(
        &self,
        visibility: MetaVisibility,
        key: String,
        value: impl AsRef<str>,
    ) -> Result<Option<MetaEntry<String>>, MetaStorageError> {
        tracing::trace!(
            visibility = ?visibility,
            value_size = %value.as_ref().as_bytes().len(),
            "setting metadata key: {key}"
        );

        if value.as_ref().as_bytes().len() > self.inner.max_value_size
            || key.as_bytes().len() > self.inner.max_key_size
        {
            return Err(MetaStorageError::SizeLimitExceeded);
        }

        let data = &mut self.inner.data.write().await;

        if !data.contains_key(&key) && data.len() >= self.inner.max_num_keys {
            return Err(MetaStorageError::SizeLimitExceeded);
        }

        let json_value = serde_json::from_str(value.as_ref())?;
        let removed = data.insert(
            key,
            MetaEntry {
                visibility,
                value: json_value,
            },
        );

        if let Some(removed) = removed {
            let removed_str = serde_json::to_string(&removed.value)?;
            Ok(Some(MetaEntry {
                visibility: removed.visibility,
                value: removed_str,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get a metadata value by key.
    ///
    /// If visibility is [`MetaVisibility::Public`], only public metadata will be returned. Other
    /// metadata will return `None`.
    pub async fn get(
        &self,
        visibility: MetaVisibility,
        key: &str,
    ) -> Result<Option<MetaEntry<String>>, MetaStorageError> {
        tracing::trace!(
            visibility = ?visibility,
            "getting metadata key: {key}"
        );

        let data = &self.inner.data.read().await;
        if let Some(value) = data.get(key)
            && visibility.can_access(value.visibility)
        {
            let value_str = serde_json::to_string(&value.value)?;
            Ok(Some(MetaEntry {
                visibility: value.visibility,
                value: serde_json::from_str(&value_str)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete a metadata key.
    pub async fn delete(
        &mut self,
        key: &str,
    ) -> Result<Option<MetaEntry<String>>, MetaStorageError> {
        let data = &mut self.inner.data.write().await;
        if let Some(removed) = data.remove(key) {
            let removed_str = serde_json::to_string(&removed.value)?;
            Ok(Some(MetaEntry {
                visibility: removed.visibility,
                value: removed_str,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all metadata key-value pairs.
    ///
    /// If visibility is [`MetaVisibility::Public`], only public metadata will be listed. Other
    /// metadata will be omitted.
    pub async fn list<T: FromIterator<(String, MetaEntry<serde_json::Value>)>>(
        &self,
        visibility: MetaVisibility,
    ) -> T {
        let data = &self.inner.data.read().await;

        data.iter()
            .filter_map(|(key, entry)| {
                if visibility.can_access(entry.visibility) {
                    Some((key.clone(), entry.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// List all metadata values. This method is similar to [`MetaStorage::list`], but only returns
    /// the values.
    ///
    /// If visibility is [`MetaVisibility::Public`], only public metadata values will be listed.
    /// Other metadata values will be omitted.
    pub async fn list_values<T: FromIterator<(String, serde_json::Value)>>(
        &self,
        visibility: MetaVisibility,
    ) -> T {
        let data = &self.inner.data.read().await;

        data.iter()
            .filter_map(|(key, entry)| {
                if visibility.can_access(entry.visibility) {
                    Some((key.clone(), entry.value.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Implementations for converting between WIT bindings and platform types.
mod conversion_impls {
    use super::*;

    impl Into<bindings::MetaVisibility> for MetaVisibility {
        fn into(self) -> bindings::MetaVisibility {
            match self {
                MetaVisibility::Public => bindings::MetaVisibility::Public,
                MetaVisibility::Private => bindings::MetaVisibility::Private,
            }
        }
    }

    impl From<bindings::MetaVisibility> for MetaVisibility {
        fn from(value: bindings::MetaVisibility) -> Self {
            match value {
                bindings::MetaVisibility::Public => MetaVisibility::Public,
                bindings::MetaVisibility::Private => MetaVisibility::Private,
            }
        }
    }

    impl Into<bindings::MetaEntry> for MetaEntry<String> {
        fn into(self) -> bindings::MetaEntry {
            bindings::MetaEntry {
                visibility: self.visibility.into(),
                value: self.value,
            }
        }
    }

    impl From<bindings::MetaEntry> for MetaEntry<String> {
        fn from(value: bindings::MetaEntry) -> Self {
            MetaEntry {
                visibility: value.visibility.into(),
                value: value.value,
            }
        }
    }
}
