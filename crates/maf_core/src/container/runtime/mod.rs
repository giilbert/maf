pub mod wasi;

use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use tokio::sync::mpsc;
use wasmtime::component::HasSelf;
use wasmtime::{self as wt};

use crate::container::{AdditionalKeyRequest, ContainerData, ContainerHandle};
use crate::server::{RoomHostImpl, UpgradeableRoomHostImpl};
use crate::wasi::bindings::AddKeyError;

#[derive(Clone)]
pub struct ContainerRuntime<R: RoomHostImpl> {
    host: R::WeakRef,
    pub(super) engine: wt::Engine,
    pub(super) linker: Arc<wt::component::Linker<ContainerData>>,
    pub(super) app_activity: &'static AtomicU64,
}

impl<R: RoomHostImpl> Debug for ContainerRuntime<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerRuntime").finish_non_exhaustive()
    }
}

impl<R: RoomHostImpl> ContainerRuntime<R> {
    pub fn init(host: R::WeakRef, app_activity: &'static AtomicU64) -> anyhow::Result<Self> {
        let engine = wt::Engine::new(wt::Config::new().epoch_interruption(true))?;
        let linker = Self::create_component_linker(&engine)?;

        Ok(Self {
            host,
            engine,
            linker: Arc::new(linker),
            app_activity,
        })
    }

    pub(super) fn create_component_linker(
        engine: &wt::Engine,
    ) -> anyhow::Result<wt::component::Linker<ContainerData>> {
        let mut linker = wt::component::Linker::new(engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
        // // TODO: Limit HTTP bandwidth?
        wasi::bindings::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

        Ok(linker)
    }

    /// Spawns a task that listens for additional room key requests from the container and handles
    /// them.
    pub(super) fn handle_additional_keys(
        &self,
        container: ContainerHandle,
        mut rx: mpsc::Receiver<AdditionalKeyRequest>,
    ) -> anyhow::Result<()> {
        let host = self.host.clone();
        let cancel_token = container.signals.cancel.clone();
        let room_id = container.room_id;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {}
                    key = rx.recv() => {
                        let host = match host.upgrade() {
                            Some(host) => host,
                            None => break,
                        };

                        let req = match key {
                            Some(key) => key,
                            None => break,
                        };

                        let res = host.room_storage().add_key(&room_id, req.key).await;
                        let _ = req.response_tx.send(res.map_err(|_| AddKeyError::Other));
                    }
                }
            }

            // TODO: handle errors
            Ok::<_, anyhow::Error>(())
        });

        Ok(())
    }
}
