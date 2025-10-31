//! Implementations of the host interfaces defined in `cobble/wit`.
//!
//! These sandboxed resources (like users, messages, requests, etc) are used by user applications
//! to interact with the outside world in a controlled manner.

mod errors;
mod hooks;
mod user;

pub use hooks::{FutureHookRequest, HookRequest};
use cobble_schemas::typed::AppSchema;
pub use user::{FutureMessageImpl, FutureUserImpl, UserImpl};
use wasmtime::component::Resource;

use crate::container::ContainerData;
use errors::ListenError;

mod generated {
    wasmtime::component::bindgen!({
        path: "../cobble/wit",
        async: true,
        with: {
            "wasi:io/poll": wasmtime_wasi_io::bindings::wasi::io::poll,
            "cobble:bindings/bindings/future-user": crate::runtime::wasi::FutureUserImpl,
            "cobble:bindings/bindings/future-message": crate::runtime::wasi::FutureMessageImpl,
            "cobble:bindings/bindings/future-hook-request": crate::runtime::wasi::FutureHookRequest,
            "cobble:bindings/bindings/user": crate::runtime::wasi::UserImpl,
            "cobble:bindings/bindings/hook-request": crate::runtime::wasi::HookRequest,
        },
        trappable_imports: true,
        trappable_error_type: {
            "cobble:bindings/bindings/listen-error" => crate::runtime::wasi::ListenError
        }
    });
}

pub use generated::Imports as Bindings;
pub use generated::cobble::bindings::bindings;

impl bindings::Host for ContainerData {
    async fn listen_user(&mut self) -> Result<Resource<FutureUserImpl>, ListenError> {
        let res = FutureUserImpl::new(self)?;
        Ok(self.resources.push(res)?)
    }

    async fn listen_hook_request(&mut self) -> Result<Resource<FutureHookRequest>, ListenError> {
        let res = FutureHookRequest::new(self)?;
        Ok(self.resources.push(res)?)
    }

    async fn report_app_schema(&mut self, schema: String) -> wasmtime::Result<()> {
        let app_schema = serde_json::from_str::<AppSchema>(&schema)
            .map_err(|_| anyhow::anyhow!("invalid app schema reported"))?;

        self.app_schema_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("app schema already reported"))?
            .send(app_schema)
            .map_err(|_| anyhow::anyhow!("failed to send app schema, receiver dropped"))?;

        Ok(())
    }

    fn convert_listen_error(&mut self, err: ListenError) -> anyhow::Result<bindings::ListenError> {
        err.0.downcast()
    }
}
