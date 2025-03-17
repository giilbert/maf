use crate::container::ContainerData;

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
use wasmtime_wasi::IoImpl;

impl HostFutureConnection for ContainerData {
    async fn drop(
        &mut self,
        rep: wasmtime::component::Resource<FutureConnection>,
    ) -> wasmtime::Result<()> {
        todo!();
    }

    async fn get(
        &mut self,
        self_: wasmtime::component::Resource<FutureConnection>,
    ) -> wasmtime::component::Resource<User> {
        todo!();
    }

    async fn subscribe(
        &mut self,
        self_: wasmtime::component::Resource<FutureConnection>,
    ) -> wasmtime::component::Resource<Pollable> {
        todo!();
    }
}

impl HostFutureRequest for ContainerData {
    async fn drop(
        &mut self,
        rep: wasmtime::component::Resource<FutureRequest>,
    ) -> wasmtime::Result<()> {
        todo!();
    }

    async fn get(&mut self, self_: wasmtime::component::Resource<FutureRequest>) -> Request {
        todo!();
    }

    async fn subscribe(
        &mut self,
        self_: wasmtime::component::Resource<FutureRequest>,
    ) -> wasmtime::component::Resource<Pollable> {
        todo!();
    }
}

impl HostUser for ContainerData {
    async fn drop(&mut self, rep: wasmtime::component::Resource<User>) -> wasmtime::Result<()> {
        todo!();
    }

    async fn new(&mut self, id: (u64, u64)) -> wasmtime::component::Resource<User> {
        todo!();
    }

    async fn send(
        &mut self,
        self_: wasmtime::component::Resource<User>,
        bytes: wasmtime::component::__internal::Vec<u8>,
    ) -> Result<(), ()> {
        todo!();
    }
}

impl generated::maf::bindings::bindings::Host for ContainerData {
    async fn next_connection(&mut self) -> wasmtime::component::Resource<FutureConnection> {
        todo!();
    }

    async fn next_request(&mut self) -> wasmtime::component::Resource<FutureRequest> {
        todo!();
    }
}
