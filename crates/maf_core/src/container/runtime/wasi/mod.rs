//! Implementations of the host interfaces defined in `maf/wit`.
//!
//! These sandboxed resources (like users, messages, requests, etc) are used by user applications
//! to interact with the outside world in a controlled manner.

mod errors;
mod hooks;
mod user;

use anyhow::Context;
use errors::ListenError;
pub use hooks::{FutureHookRequest, HookRequest};
use maf_schemas::apps::{JsonMetaEntry, MAX_ROOM_KEY_LENGTH, MetaVisibility};
use maf_schemas::typed::AppSchema;
use tokio::sync::oneshot;
pub use user::{FutureMessageImpl, FutureUserImpl, UserImpl};
use wasmtime::component::Resource;

use crate::container::{AdditionalKeyRequest, ContainerData, MAX_ADDITIONAL_KEYS};

mod generated {
    use crate::container::runtime::wasi as wasi_impl;

    wasmtime::component::bindgen!({
        path: "../maf/wit",
        world: "imports",
        imports: { default: async | trappable },
        exports: { default: async },
        with: {
            "wasi:io/poll": wasmtime_wasi_io::bindings::wasi::io::poll,
            "maf:bindings/bindings.future-user": wasi_impl::FutureUserImpl,
            "maf:bindings/bindings.future-message": wasi_impl::FutureMessageImpl,
            "maf:bindings/bindings.future-hook-request": wasi_impl::FutureHookRequest,
            "maf:bindings/bindings.user": wasi_impl::UserImpl,
            "maf:bindings/bindings.hook-request": wasi_impl::HookRequest,
        },
        trappable_error_type: {
            "maf:bindings/bindings.listen-error" => wasi_impl::ListenError
        },
        anyhow: true,
    });
}

pub use generated::Imports as Bindings;
pub(crate) use generated::maf::bindings::bindings;

fn serialize_meta_entry(
    entry: Option<JsonMetaEntry>,
) -> anyhow::Result<Option<bindings::MetaEntry>> {
    match entry {
        Some(json_entry) => {
            let meta_entry = json_entry
                .serialize()
                .context("failed to serialize meta entry")?;
            Ok(Some(meta_entry.into()))
        }
        None => Ok(None),
    }
}

impl bindings::Host for ContainerData {
    async fn listen_user(&mut self) -> Result<Resource<FutureUserImpl>, ListenError> {
        let res = FutureUserImpl::new(self)?;
        self.signals.readied.cancel();
        Ok(self.resources.push(res)?)
    }

    async fn listen_hook_request(&mut self) -> Result<Resource<FutureHookRequest>, ListenError> {
        let res = FutureHookRequest::new(self)?;
        Ok(self.resources.push(res)?)
    }

    async fn report_app_schema(&mut self, schema: String) -> anyhow::Result<()> {
        let app_schema = serde_json::from_str::<AppSchema>(&schema)
            .map_err(|_| anyhow::anyhow!("invalid app schema reported"))?;

        self.app_schema_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("app schema already reported"))?
            .send(app_schema)
            .map_err(|_| anyhow::anyhow!("failed to send app schema, receiver dropped"))?;

        Ok(())
    }

    // TODO: Error handling
    async fn get_meta(&mut self, key: String) -> anyhow::Result<Option<bindings::MetaEntry>> {
        self.meta
            .get(MetaVisibility::Private, &key)
            .await
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(serialize_meta_entry)
    }

    async fn set_meta(
        &mut self,
        visibility: bindings::MetaVisibility,
        key: String,
        value: String,
    ) -> anyhow::Result<Option<bindings::MetaEntry>> {
        self.meta
            .set(
                match visibility {
                    bindings::MetaVisibility::Public => MetaVisibility::Public,
                    bindings::MetaVisibility::Private => MetaVisibility::Private,
                },
                key,
                value,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(serialize_meta_entry)
    }

    async fn delete_meta(&mut self, key: String) -> anyhow::Result<Option<bindings::MetaEntry>> {
        self.meta
            .delete(&key)
            .await
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(serialize_meta_entry)
    }

    async fn list_meta(&mut self) -> anyhow::Result<Vec<(String, bindings::MetaEntry)>> {
        self.meta
            .list::<Vec<(String, JsonMetaEntry)>>(MetaVisibility::Private)
            .await
            .into_iter()
            .map(|(key, entry)| {
                let meta_entry = entry.serialize()?;
                Ok((key, meta_entry.into()))
            })
            .collect::<Result<Vec<(String, bindings::MetaEntry)>, serde_json::Error>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn add_key(&mut self, key: String) -> anyhow::Result<Result<(), bindings::AddKeyError>> {
        if self.num_additional_keys >= MAX_ADDITIONAL_KEYS {
            return Ok(Err(bindings::AddKeyError::MaxKeysReached));
        }

        if key.is_empty() || key.len() > MAX_ROOM_KEY_LENGTH {
            return Ok(Err(bindings::AddKeyError::InvalidKey));
        }

        let (response_tx, response_rx) = oneshot::channel();
        self.num_additional_keys += 1;

        match self
            .add_additional_keys_tx
            .send(AdditionalKeyRequest { key, response_tx })
            .await
        {
            Ok(_) => {}
            Err(_) => {
                return Ok(Err(bindings::AddKeyError::Other));
            }
        };

        match response_rx.await {
            Ok(result) => Ok(result),
            Err(_) => Ok(Err(bindings::AddKeyError::Other)),
        }
    }

    fn convert_listen_error(&mut self, err: ListenError) -> anyhow::Result<bindings::ListenError> {
        err.0.downcast().map_err(|e| anyhow::anyhow!(e))
    }
}
