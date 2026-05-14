use std::{pin::Pin, task::Poll};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::{io::AsyncWrite, sync::mpsc};
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};

/// A factory for creating [`ContainerStdout`]. This is needed because WASI uses factory-pattern
/// for creating custom stdout streams.
#[derive(Debug, Clone)]
pub struct ContainerStdoutFactory {
    output_tx: mpsc::Sender<String>,
}

impl ContainerStdoutFactory {
    pub fn new(output_tx: mpsc::Sender<String>) -> Self {
        Self { output_tx }
    }
}

impl IsTerminal for ContainerStdoutFactory {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for ContainerStdoutFactory {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
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

impl AsyncWrite for ContainerStdout {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.buffer_length += buf.len();
        self.buffer.push(Bytes::copy_from_slice(buf));
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let buffer = self.buffer.concat();
        let string = String::from_utf8_lossy(&buffer);

        self.buffer.drain(..);
        self.buffer_length = 0;

        self.line_buffer += &string;

        // Send complete lines to the output channel
        while let Some(pos) = self.line_buffer.find('\n') {
            let line = self.line_buffer.drain(..=pos).collect::<String>();
            self.output_tx.try_send(line).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "failed to send output")
            })?;
        }

        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl wasmtime_wasi::p2::Pollable for ContainerStdout {
    async fn ready(&mut self) {}
}
