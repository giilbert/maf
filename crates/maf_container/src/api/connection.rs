use std::{any, time::Duration};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::{sync::mpsc, time::timeout};
use tokio_tungstenite::tungstenite::Utf8Bytes;
use uuid::Uuid;

use super::gateway::ConnectQueryParams;

pub struct Connection {
    takeable: Option<TakeableConnection>,
    shared: ConnectionHandle,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    id: Uuid,
    command_tx: mpsc::Sender<ConnectionCommand>,
}

struct TakeableConnection {
    command_rx: mpsc::Receiver<ConnectionCommand>,
    command_tx: mpsc::Sender<ConnectionCommand>,

    ws_rx: SplitStream<WebSocket>,
    ws_tx: SplitSink<WebSocket, Message>,
}

enum ConnectionCommand {
    Close,
}

impl Connection {
    pub async fn init(
        ws: WebSocket,
        connect_query_params: ConnectQueryParams,
    ) -> anyhow::Result<Self> {
        let (mut ws_tx, mut ws_rx) = ws.split();
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
            },
            takeable: Some(TakeableConnection {
                command_rx,
                command_tx,
                ws_rx,
                ws_tx,
            }),
        })
    }

    pub fn handle(&self) -> ConnectionHandle {
        self.shared.clone()
    }

    fn handle_websocket_message(&self, message: Message) -> anyhow::Result<()> {
        match message {
            Message::Text(data) => {
                let (packet_type, data) = data
                    .split_once(":")
                    .ok_or_else(|| anyhow::anyhow!("invalid message format, expected type:data"))?;

                // println!("got message type: {packet_type:?}");
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
                            self.handle_websocket_message(message)?;
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
                    }
                }
            }
        }

        Ok(())
    }
}
