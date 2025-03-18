use crate::container::ContainerData;
use wasmtime::component::Resource;

mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        async: true,
        with: {
            // "wasi:io/poll/pollable": wasmtime_wasi_io::poll::DynPollable,
            "wasi:io/poll": wasmtime_wasi_io::bindings::wasi::io::poll,
        }
    });
}

pub use generated::maf::bindings::bindings::*;
pub use generated::Imports as Bindings;

impl HostFutureConnection for ContainerData {
    async fn drop(&mut self, connection: Resource<FutureConnection>) -> wasmtime::Result<()> {
        todo!();
    }

    async fn get(&mut self, connection: Resource<FutureConnection>) -> Resource<User> {
        todo!();
    }

    async fn subscribe(&mut self, connection: Resource<FutureConnection>) -> Resource<Pollable> {
        todo!();
    }
}

impl HostFutureRequest for ContainerData {
    async fn drop(&mut self, request: Resource<FutureRequest>) -> wasmtime::Result<()> {
        todo!();
    }

    async fn get(&mut self, request: Resource<FutureRequest>) -> Request {
        todo!();
    }

    async fn subscribe(&mut self, request: Resource<FutureRequest>) -> Resource<Pollable> {
        todo!();
    }
}

impl HostUser for ContainerData {
    async fn drop(&mut self, user: Resource<User>) -> wasmtime::Result<()> {
        todo!();
    }

    async fn new(&mut self, id: (u64, u64)) -> Resource<User> {
        todo!();
    }

    async fn send(&mut self, user: Resource<User>, bytes: Vec<u8>) -> Result<(), ()> {
        todo!();
    }
}

impl Host for ContainerData {
    async fn next_connection(&mut self) -> Resource<FutureConnection> {
        todo!();
    }

    async fn next_request(&mut self) -> Resource<FutureRequest> {
        todo!();
    }
}
