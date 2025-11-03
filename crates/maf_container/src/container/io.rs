use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;
use wasmtime_wasi::p2::{OutputStream, StdoutStream, StreamResult};

/// A factory for creating [`ContainerStdout`]. This is needed because WASI uses factory-pattern
/// for creating custom stdout streams.
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

/// A custom stdout stream that conveniently buffers user-generated output and sends it to the
/// provided channel line-by-line.
pub struct ContainerStdout {
    /// Buffered output chunks. Does not output anything until `.flush()` is called.
    buffer: Vec<Bytes>,
    buffer_length: usize,
    /// Used to store incomplete lines between flushes.
    line_buffer: String,
    /// Channel to send output lines to.
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
        // Move the current buffer to a string and clear it
        let buffer = self.buffer.concat();
        let string = String::from_utf8_lossy(&buffer);

        self.buffer.drain(..);
        self.buffer_length = 0;

        self.line_buffer += &string;

        // Send complete lines to the output channel
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
