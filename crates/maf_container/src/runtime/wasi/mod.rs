use wasmtime::component::bindgen;

use crate::container::ContainerData;

bindgen!({
    world: "bindings",
    path: "src/runtime/wasi/maf.wit",
    async: true,
});

impl BindingsImports for ContainerData {
    async fn print(&mut self, message: String) -> Result<(), ()> {
        self.output_tx.try_send(message).map_err(|_| ())?;
        Ok(())
    }
}
