use wasmtime::component::bindgen;

use crate::container::ContainerData;

bindgen!({
    world: "bindings",
    path: "src/runtime/wasi/maf.wit",
    async: true,
});

// impl BindingsImports for ContainerData {}
