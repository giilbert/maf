mod errors;
mod user;

pub use user::{FutureMessage, FutureUser, User};
use wasmtime::component::Resource;

use crate::container::ContainerData;
use errors::ListenError;

mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        async: true,
        with: {
            "wasi:io/poll": wasmtime_wasi_io::bindings::wasi::io::poll,
            "maf:bindings/bindings/future-user": crate::runtime::wasi::FutureUser,
            "maf:bindings/bindings/future-message": crate::runtime::wasi::FutureMessage,
            "maf:bindings/bindings/user": crate::runtime::wasi::User
        },
        trappable_imports: true,
        trappable_error_type: {
            "maf:bindings/bindings/listen-error" => crate::runtime::wasi::ListenError,
        }
    });
}

pub use generated::Imports as Bindings;
pub use generated::maf::bindings::bindings;

impl bindings::Host for ContainerData {
    async fn listen_user(&mut self) -> Result<Resource<FutureUser>, ListenError> {
        let res = FutureUser::new(self)?;
        Ok(self.resources.push(res)?)
    }

    fn convert_listen_error(&mut self, err: ListenError) -> anyhow::Result<bindings::ListenError> {
        err.0.downcast()
    }
}
