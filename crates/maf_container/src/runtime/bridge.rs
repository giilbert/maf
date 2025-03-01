use std::sync::atomic::AtomicI32;

use crate::container::ContainerData;
use wasmtime::{self as wt};

use super::ContainerRuntime;

static NUMBER: AtomicI32 = AtomicI32::new(0);

pub(crate) async fn wasm_fn_print(
    mut caller: wt::Caller<'_, ContainerData>,
    params: (u32, u64),
) -> anyhow::Result<()> {
    let (ptr, len) = params;

    let message = {
        let memory = caller
            .get_export("memory")
            .ok_or_else(|| anyhow::anyhow!("failed to find `memory` export on caller"))?
            .into_memory()
            .ok_or_else(|| anyhow::anyhow!("`memory` export on caller is not a memory"))?;

        let data = memory
            .data(&caller)
            .get(ptr as usize..)
            .ok_or_else(|| anyhow::anyhow!("failed to get data from memory at offset {}", ptr))?;

        let data = &data[..len as usize];

        String::from_utf8_lossy(data).to_string()
    };

    caller
        .data_mut()
        .output_tx
        .send(message.to_string())
        .await
        .map_err(|_| anyhow::anyhow!("failed to send message to output channel"))?;

    Ok(())
}

impl ContainerRuntime {
    pub(super) fn create_linker_with_ffi(
        engine: &wt::Engine,
    ) -> anyhow::Result<wt::Linker<ContainerData>> {
        let mut linker = wt::Linker::new(engine);
        linker.func_wrap_async("maf", "ffi_print", |caller, params: (u32, u64)| {
            Box::new(wasm_fn_print(caller, params))
        })?;
        Ok(linker)
    }
}
