use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;
use wasmtime_wasi::p2::{OutputStream, StdoutStream, StreamResult};

#[derive(Debug, Clone)]
pub struct ContainerStdoutFactory {
    pub(super) output_tx: mpsc::Sender<String>,
}

impl StdoutStream for ContainerStdoutFactory {
    fn isatty(&self) -> bool {
        false
    }

    fn stream(&self) -> Box<dyn wasmtime_wasi::p2::OutputStream> {
        Box::new(ContainerStdout {
            buffer_length: 0,
            buffer: Vec::new(),
            line_buffer: String::new(),
            output_tx: self.output_tx.clone(),
        })
    }
}

pub struct ContainerStdout {
    buffer_length: usize,
    buffer: Vec<Bytes>,
    line_buffer: String,
    pub(super) output_tx: mpsc::Sender<String>,
}

// FIXME: limit buffer size
impl OutputStream for ContainerStdout {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        self.buffer_length += bytes.len();
        self.buffer.push(bytes);

        Ok(())
    }

    fn flush(&mut self) -> wasmtime_wasi::p2::StreamResult<()> {
        let buffer = self.buffer.concat();
        let string = String::from_utf8_lossy(&buffer);

        self.buffer.drain(..);
        self.buffer_length = 0;

        self.line_buffer += &string;

        while let Some(pos) = self.line_buffer.find('\n') {
            let line = self.line_buffer.drain(..=pos).collect::<String>();
            self.output_tx
                .try_send(line)
                .map_err(|_| wasmtime_wasi::p2::StreamError::Closed)?;
        }

        Ok(())
    }

    fn check_write(&mut self) -> wasmtime_wasi::p2::StreamResult<usize> {
        Ok(usize::MAX)
    }
}

#[async_trait]
impl wasmtime_wasi::p2::Pollable for ContainerStdout {
    async fn ready(&mut self) {}
}
