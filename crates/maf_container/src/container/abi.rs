use super::Container;

/// A section of contiguous memory in the WASM module.
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct WasmBuffer {
    pub(crate) start: u32,
    pub(crate) size: u32,
    pub(crate) align: u32,
    alive: bool,
}

impl Container {
    /// Allocates a memory buffer in the WASM module and returns the pointer to it.
    pub fn alloc(&mut self, size: u32, align: u32) -> anyhow::Result<WasmBuffer> {
        let start = self.exports.alloc.call(&mut self.store, (size, align))?;
        Ok(WasmBuffer {
            start,
            size,
            align,
            alive: true,
        })
    }

    /// Deallocates a memory buffer in the WASM module.
    pub fn dealloc(&mut self, mut buffer: WasmBuffer) -> anyhow::Result<()> {
        self.exports
            .dealloc
            .call(&mut self.store, (buffer.start, buffer.size, buffer.align))?;
        buffer.alive = false;
        Ok(())
    }

    pub fn alloc_string(&mut self, s: &str) -> anyhow::Result<WasmBuffer> {
        let buffer = self.alloc(s.len() as u32, 1)?;
        self.get_memory()?
            .write(&mut self.store, buffer.start as usize, s.as_bytes())?;
        Ok(buffer)
    }
}

impl Drop for WasmBuffer {
    fn drop(&mut self) {
        if self.alive {
            println!("[warn] WasmBuffer was not deallocated before dropping");
        }
    }
}
