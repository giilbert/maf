use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    extract::{
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use base64::Engine;
use bytes::Bytes;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use maf_schemas::{
    apps::ConnectQueryParams,
    error::ErrorResponse,
    packet::{ServerHandshake, TxPacket},
    project_config::AuthMode,
};
use tokio::{
    sync::{Mutex, mpsc},
    time::timeout,
};
use uuid::Uuid;

use crate::{server::RoomInner, wasi::bindings};

pub struct Connection {
    takeable: Option<TakeableConnection>,
    shared: ConnectionHandle,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    pub(crate) id: Uuid,
    pub(crate) auth_data: Option<serde_json::Value>,
    message_rx: Arc<Mutex<Option<mpsc::Receiver<bindings::Message>>>>,
    command_tx: mpsc::Sender<ConnectionCommand>,
}

struct TakeableConnection {
    command_rx: mpsc::Receiver<ConnectionCommand>,
    // command_tx: mpsc::Sender<ConnectionCommand>,
    message_tx: mpsc::Sender<bindings::Message>,

    ws_rx: SplitStream<WebSocket>,
    ws_tx: SplitSink<WebSocket, Message>,
}

pub enum ConnectionCommand {
    Close,
    Send(bindings::Message),
    SendPong(Bytes),
}

impl Connection {
    pub async fn init(ws: WebSocket, auth_data: Option<serde_json::Value>) -> anyhow::Result<Self> {
        let (mut ws_tx, mut ws_rx) = ws.split();
        let (message_tx, message_rx) = mpsc::channel::<bindings::Message>(100);
        let (command_tx, command_rx) = mpsc::channel::<ConnectionCommand>(100);

        let connection_id = Uuid::new_v4();

        match timeout(Duration::from_secs(1), ws_rx.next()).await {
            Ok(Some(Ok(Message::Text(_message)))) => {
                ws_tx
                    .send(Message::Text(
                        serde_json::to_string(&TxPacket::Handshake::<()>(ServerHandshake {
                            id: connection_id,
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
                auth_data,
                command_tx: command_tx.clone(),
                message_rx: Arc::new(Mutex::new(Some(message_rx))),
            },
            takeable: Some(TakeableConnection {
                command_rx,
                // command_tx,
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

        let commands = self.shared.command_tx.clone();

        loop {
            tokio::select! {
                message = &mut takeable.ws_rx.next() => {
                    match message {
                        Some(Ok(Message::Ping(frame))) => {
                            commands.send(ConnectionCommand::SendPong(frame)).await?;
                        }
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
                            if let Err(error) = takeable.ws_tx.send(
                                convert_to_axum_message(message)).await {
                                tracing::warn!("an error occurred sending WebSocket message: {error:?}");
                            }
                        },
                        Some(ConnectionCommand::SendPong(frame)) => {
                            if let Err(error) = takeable.ws_tx.send(Message::Pong(frame)).await {
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

#[async_trait]
impl crate::Connection for ConnectionHandle {
    fn id(&self) -> Uuid {
        self.id
    }

    fn send(&mut self, message: bindings::Message) -> Result<(), bindings::SendError> {
        self.command_tx
            .try_send(ConnectionCommand::Send(message))
            .map_err(|e| match e {
                mpsc::error::TrySendError::Closed(_) => bindings::SendError::Closed,
                mpsc::error::TrySendError::Full(_) => bindings::SendError::BufferFull,
            })
    }

    fn auth(&self) -> Option<&serde_json::Value> {
        self.auth_data.as_ref()
    }

    async fn get_message_channel(&self) -> anyhow::Result<mpsc::Receiver<bindings::Message>> {
        self.message_rx
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("message receiver has already been taken"))
    }
}

fn convert_to_axum_message(message: bindings::Message) -> Message {
    match message {
        bindings::Message::Text(text) => Message::Text(text.into()),
        bindings::Message::Binary(data) => Message::Binary(data.into()),
    }
}

pub fn pre_create_room_auth_check(
    query_params: &ConnectQueryParams,
    auth_mode: Option<&AuthMode>,
) -> Result<(), ErrorResponse> {
    if let Some(AuthMode::Jwt) = auth_mode
        && query_params.token.is_none()
    {
        return Err(ErrorResponse::unauthorized(Some(
            "This room requires a JWT token for authentication.",
        )));
    }

    const MAX_TOKEN_LENGTH: usize = 8192; // 8 KB
    if query_params
        .token
        .as_ref()
        .is_some_and(|t| t.len() > MAX_TOKEN_LENGTH)
    {
        return Err(ErrorResponse::bad_request(Some(
            "Token length exceeds maximum allowed size.",
        )));
    }

    Ok(())
}

pub fn get_auth_data(
    query_params: &ConnectQueryParams,
    auth_mode: Option<&AuthMode>,
    room: &RoomInner,
) -> Result<Option<serde_json::Value>, ErrorResponse> {
    match &query_params.token {
        Some(token) => {
            // If the mode is JWT, we need to decode and verify the token
            if let Some(AuthMode::Jwt) = auth_mode {
                let decoded = room.decode_token(&token).map_err(|e| {
                    ErrorResponse::unauthorized(Some(&format!("invalid token: {}", e)))
                })?;

                Ok(Some(decoded))
            } else {
                // If the auth mode is not JWT, first base64 decode the token and then parse as JSON
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(&token)
                    .map_err(|e| {
                        ErrorResponse::bad_request(Some(&format!("failed to decode token: {}", e)))
                    })?;
                let decoded: serde_json::Value = serde_json::from_slice(&decoded).map_err(|e| {
                    ErrorResponse::bad_request(Some(&format!("failed to parse token: {}", e)))
                })?;

                Ok(Some(decoded))
            }
        }
        None => Ok(None),
    }
}

pub async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    room: RoomInner,
    auth_data: Option<serde_json::Value>,
) -> Response {
    ws.on_upgrade(|ws| async move {
        tracing::debug!(
            auth_data = ?auth_data,
            "connecting websocket to room {}", room.id()
        );

        let try_init = |ws: WebSocket, room: RoomInner| async move {
            let connection = Connection::init(ws, auth_data).await?;
            let handle = connection.handle();
            room.add_connection(handle).await?;
            Ok::<_, anyhow::Error>(connection)
        };

        let connection = match try_init(ws, room).await {
            Ok(connection) => connection,
            Err(error) => {
                tracing::warn!("failed to initialize connection: {error:?}");
                return;
            }
        };

        match connection.run().await {
            Ok(_) => tracing::debug!("websocket connection closed"),
            Err(error) => tracing::warn!("websocket connection error: {error:?}"),
        }
    })
}
