use crate::container::connection::ConnectionHandle;
use crate::container::ContainerData;
use tokio::sync::mpsc;
use wasmtime::{component::Resource, Trap};

mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        async: true,
        with: {
            "wasi:io/poll": wasmtime_wasi_io::bindings::wasi::io::poll,
            "maf:bindings/bindings/future-connection": crate::runtime::wasi::FutureConnection
        },
        trappable_imports: true,
        trappable_error_type: {
            "maf:bindings/bindings/listen-error" => crate::runtime::wasi::ListenError,
        }
    });
}

pub use generated::maf::bindings::bindings::{self, *};
pub use generated::Imports as Bindings;

pub struct FutureConnection {
    channel: mpsc::Receiver<ConnectionHandle>,
}

pub struct ListenError(pub wasmtime_wasi::TrappableError<bindings::ListenError>);

impl From<wasmtime::component::ResourceTableError> for ListenError {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self(wasmtime_wasi::TrappableError::trap(error))
    }
}

impl HostFutureConnection for ContainerData {
    async fn drop(&mut self, connection: Resource<FutureConnection>) -> anyhow::Result<()> {
        todo!();
    }

    async fn get(
        &mut self,
        connection: Resource<FutureConnection>,
    ) -> Result<Resource<User>, ListenError> {
        let connection = self.resources.get_mut(&connection)?;
        todo!();
    }

    async fn subscribe(
        &mut self,
        connection: Resource<FutureConnection>,
    ) -> Result<Resource<Pollable>, ListenError> {
        todo!();
    }
}

impl HostFutureRequest for ContainerData {
    async fn drop(&mut self, request: Resource<FutureRequest>) -> anyhow::Result<()> {
        todo!();
    }

    async fn get(&mut self, request: Resource<FutureRequest>) -> Result<Request, ListenError> {
        todo!();
    }

    async fn subscribe(
        &mut self,
        request: Resource<FutureRequest>,
    ) -> Result<Resource<Pollable>, ListenError> {
        todo!();
    }
}

impl HostUser for ContainerData {
    async fn drop(&mut self, user: Resource<User>) -> anyhow::Result<()> {
        todo!();
    }

    async fn new(&mut self, id: (u64, u64)) -> anyhow::Result<Resource<User>> {
        todo!();
    }

    async fn send(
        &mut self,
        user: Resource<User>,
        bytes: Vec<u8>,
    ) -> anyhow::Result<Result<(), ()>> {
        todo!();
    }
}

impl Host for ContainerData {
    async fn listen_connection(&mut self) -> Result<Resource<FutureConnection>, ListenError> {
        todo!();
    }

    async fn listen_request(&mut self) -> Result<Resource<FutureRequest>, ListenError> {
        todo!();
    }

    fn convert_listen_error(&mut self, err: ListenError) -> anyhow::Result<bindings::ListenError> {
        err.0.downcast()
    }
}
