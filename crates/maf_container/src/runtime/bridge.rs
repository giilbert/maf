use crate::container::ContainerData;
use wasmtime::{
    self as wt,
    component::{HasData, HasSelf},
};
use wasmtime_wasi_http::WasiHttpImpl;
use wasmtime_wasi_io::IoImpl;

use super::{ContainerRuntime, wasi};

struct WasiHttp<T>(T);

impl<T: 'static> HasData for WasiHttp<T> {
    type Data<'a> = WasiHttpImpl<&'a mut T>;
}

impl ContainerRuntime {
    pub(super) fn create_component_linker(
        engine: &wt::Engine,
    ) -> anyhow::Result<wt::component::Linker<ContainerData>> {
        let mut linker = wt::component::Linker::new(engine);

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        // TODO: Limit HTTP bandwidth?
        wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker::<_, WasiHttp<_>>(
            &mut linker,
            |state| WasiHttpImpl(IoImpl(state)),
        )?;
        wasi::bindings::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

        Ok(linker)
    }
}
