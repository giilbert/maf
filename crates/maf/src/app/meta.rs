//! An interface to inactive with the MAF Meta API.

use std::{collections::HashMap, ops::Deref, sync::Arc};

use maf_schemas::apps::{MetaEntry, MetaVisibility};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    callable::{BoxedCallable, CallableFetch},
    platform::{Platform, TargetPlatform},
    App,
};

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MetaKey(pub(crate) Arc<str>);

impl Deref for MetaKey {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An interace that holds entries for the MAF Meta API.
#[derive(Default)]
pub struct MetaStorage {
    /// The `platform` property of [`MetaStorage`] is only needed when actually modifying values and
    /// interacting with the host platform. However, [`MetaStorage`] needs to be present during the
    /// build stage of the application to set meta updaters. In this case, `platform` will be
    /// initialized to [`None`].
    pub(crate) platform: Option<Arc<TargetPlatform>>,
    pub(crate) updaters: HashMap<MetaKey, AnyMetaUpdater>,
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
    /// Gets the instance of a [`Platform`] this storage (and app) is bound to, panicking if it is
    /// not initialized.
    fn platform(&self) -> &TargetPlatform {
        self.platform
            .as_ref()
            .expect("MetaStorage was not initialized with a platform")
    }

    /// Sets a meta entry with the specified key and value, returning the previous entry if it
    /// existed.
    pub fn set(
        &self,
        visibility: MetaVisibility,
        key: &str,
        value: impl Serialize,
    ) -> Result<Option<MetaEntry>, MetaError> {
        Ok(self.platform().set_meta(
            visibility,
            key,
            &serde_json::to_string(&value).map_err(MetaError::SerializationError)?,
        ))
    }

    /// Gets a meta entry with the specified key, deserialized into the specified type.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<T, MetaError> {
        if let Some(entry) = self.platform().get_meta(key) {
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
        self.platform().get_meta(key)
    }

    /// Deletes a meta entry with the specified key, returning the previous entry if it existed.
    pub fn delete(&self, key: &str) -> Option<MetaEntry> {
        self.platform().delete_meta(key)
    }

    /// Lists all meta entries.
    pub fn list(&self) -> Vec<(String, MetaEntry)> {
        self.platform().list_meta()
    }

    /// Updates all meta entries by triggering their updaters.
    pub(crate) async fn update_all_meta(&self, app: App) -> anyhow::Result<()> {
        for name in self.updaters.keys() {
            self.trigger_meta_update(app.clone(), name).await?;
        }

        Ok(())
    }

    pub(crate) async fn trigger_meta_update(&self, app: App, name: &MetaKey) -> anyhow::Result<()> {
        if let Some(updater) = self.updaters.get(name) {
            let ctx = MetaContext { app };
            let value = (updater.create)(ctx).await?;
            self.set(updater.visibility, name, value)?;
        }

        Ok(())
    }
}

/// Support for meta callables
pub struct MetaContext {
    app: App,
}

impl CallableFetch<App> for MetaContext {
    fn fetch(&self) -> App {
        self.app.clone()
    }
}

pub struct AnyMetaUpdater {
    pub(crate) _key: MetaKey,
    pub(crate) visibility: MetaVisibility,
    pub(crate) create: BoxedCallable<MetaContext, serde_json::Value, serde_json::Error>,
}
