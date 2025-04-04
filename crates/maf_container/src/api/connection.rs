use std::{any, sync::Arc, time::Duration};

use axum::extract::ws::{Message, WebSocket};
use bytes::Bytes;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::{
    sync::{mpsc, Mutex},
    time::timeout,
};
use tokio_tungstenite::tungstenite::Utf8Bytes;
use uuid::Uuid;

use crate::runtime::wasi::bindings;

use super::gateway::ConnectQueryParams;

pub struct Connection {
    takeable: Option<TakeableConnection>,
    shared: ConnectionHandle,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    id: Uuid,
    message_rx: Arc<Mutex<Option<mpsc::Receiver<bindings::Message>>>>,
    command_tx: mpsc::Sender<ConnectionCommand>,
}

struct TakeableConnection {
    command_rx: mpsc::Receiver<ConnectionCommand>,
    command_tx: mpsc::Sender<ConnectionCommand>,

    message_tx: mpsc::Sender<bindings::Message>,

    ws_rx: SplitStream<WebSocket>,
    ws_tx: SplitSink<WebSocket, Message>,
}

enum ConnectionCommand {
    Close,
    Send(Message),
}

impl Connection {
    pub async fn init(
        ws: WebSocket,
        connect_query_params: ConnectQueryParams,
    ) -> anyhow::Result<Self> {
        let (mut ws_tx, mut ws_rx) = ws.split();
        let (message_tx, message_rx) = mpsc::channel::<bindings::Message>(100);
        let (command_tx, command_rx) = mpsc::channel::<ConnectionCommand>(100);

        let connection_id = Uuid::new_v4();

        match timeout(Duration::from_secs(1), ws_rx.next()).await {
            Ok(Some(Ok(Message::Text(message)))) => {
                ws_tx
                    .send(Message::Text(
                        serde_json::to_string(&serde_json::json!({
                            "type": "handshake",
                            "data": {
                                "id": connection_id,
                            }
                        }))?
                        .into(),
                    ))
                    .await?;

                // TODO: take auth payload and do something
            }
            rest => {
                let mut ws = ws_tx.reunite(ws_rx)?;
                ws.close().await?;

                match rest {
                    Ok(Some(Ok(_))) => anyhow::bail!("invalid type received during handshake"),
                    Ok(Some(Err(error))) => anyhow::bail!("{error:?}"),
                    Ok(None) => anyhow::bail!("websocket closed during handshake"),
                    Err(_) => anyhow::bail!("websocket handshake timed out"),
                }
            }
        }

        Ok(Self {
            shared: ConnectionHandle {
                id: connection_id,
                command_tx: command_tx.clone(),
                message_rx: Arc::new(Mutex::new(Some(message_rx))),
            },
            takeable: Some(TakeableConnection {
                command_rx,
                command_tx,
                message_tx,
                ws_rx,
                ws_tx,
            }),
        })
    }

    pub fn handle(&self) -> ConnectionHandle {
        self.shared.clone()
    }

    async fn handle_websocket_message(
        &self,
        message_tx: &mpsc::Sender<bindings::Message>,
        message: Message,
    ) -> anyhow::Result<()> {
        match message {
            Message::Text(data) => {
                let (packet_type, data) = data
                    .split_once(":")
                    .ok_or_else(|| anyhow::anyhow!("invalid message format, expected type:data"))?;

                // TODO: is there a way to pass data without copying it?
                message_tx
                    .send(bindings::Message::Text(data.to_string()))
                    .await?;
            }
            Message::Binary(data) => {
                message_tx
                    .send(bindings::Message::Binary(data.into_iter().collect()))
                    .await?;
            }
            Message::Close(close_frame) => {
                tracing::debug!("got close frame: {close_frame:?}");
                return Ok(());
            }
            _ => anyhow::bail!("invalid message type"),
        };

        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut takeable = self
            .takeable
            .take()
            .ok_or_else(|| anyhow::anyhow!("connection has already been taken"))?;

        loop {
            tokio::select! {
                message = &mut takeable.ws_rx.next() => {
                    match message {
                        Some(Ok(message)) => {
                            self.handle_websocket_message(&takeable.message_tx, message).await?;
                        }
                        Some(Err(error)) => {
                            tracing::warn!("an error occurred receiving WebSocket message: {error:?}");
                        }
                        None => break
                    }
                },

                command = takeable.command_rx.recv() => {
                    match command {
                        Some(ConnectionCommand::Close) | None => break,
                        Some(ConnectionCommand::Send(message)) => {
                            if let Err(error) = takeable.ws_tx.send(message).await {
                                tracing::warn!("an error occurred sending WebSocket message: {error:?}");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl ConnectionHandle {
    pub async fn send(&self, message: Message) -> anyhow::Result<()> {
        self.command_tx
            .send(ConnectionCommand::Send(message))
            .await
            .map_err(|_| anyhow::anyhow!("failed to send command"))?;

        Ok(())
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        self.command_tx
            .send(ConnectionCommand::Close)
            .await
            .map_err(|_| anyhow::anyhow!("failed to send command"))?;

        Ok(())
    }

    pub async fn take_message_rx(&self) -> anyhow::Result<mpsc::Receiver<bindings::Message>> {
        self.message_rx
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("message receiver has already been taken"))
    }
}
