use std::{future::Future, pin::Pin};

use bytes::Bytes;
use tokio::sync::mpsc;
use wasmtime_wasi::{async_trait, bindings::io::streams::HostOutputStream, StdoutStream};

#[derive(Debug, Clone)]
pub struct ContainerStdoutFactory {
    pub(super) output_tx: mpsc::Sender<String>,
}

impl StdoutStream for ContainerStdoutFactory {
    fn isatty(&self) -> bool {
        false
    }

    fn stream(&self) -> Box<dyn wasmtime_wasi::OutputStream> {
        Box::new(ContainerStdout {
            buffer_length: 0,
            buffer: Vec::new(),
            output_tx: self.output_tx.clone(),
        })
    }
}

pub struct ContainerStdout {
    buffer_length: usize,
    buffer: Vec<Bytes>,
    pub(super) output_tx: mpsc::Sender<String>,
}

// FIXME: limit buffer size
impl HostOutputStream for ContainerStdout {
    fn write(
        &mut self,
        self_: wasmtime::component::Resource<wasmtime_wasi::DynOutputStream>,
        contents: Vec<u8>,
    ) -> Result<(), wasmtime_wasi_io::bindings::wasi::io::super::super::_TrappableError0> {
        self.buffer_length += contents.len();
        self.buffer.push(bytes);
        Ok(())
    }

    fn flush(&mut self) -> wasmtime_wasi::StreamResult<()> {
        let string = String::from_utf8_lossy(&self.buffer.concat()).to_string();

        self.buffer.drain(..);
        self.buffer_length = 0;

        self.output_tx
            .try_send(string)
            .map_err(|_| wasmtime_wasi::StreamError::Closed)?;
        Ok(())
    }

    fn check_write(&mut self) -> wasmtime_wasi::StreamResult<usize> {
        Ok(usize::MAX)
    }
}

#[async_trait]
impl wasmtime_wasi::Subscribe for ContainerStdout {
    async fn ready(&mut self) {}
}
