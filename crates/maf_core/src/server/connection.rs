use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{Message, WebSocket};
use axum::response::Response;
use base64::Engine;
use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use maf_schemas::apps::ConnectQueryParams;
use maf_schemas::error::ErrorResponse;
use maf_schemas::packet::{ServerHandshake, TxPacket};
use maf_schemas::project_config::AuthMode;
use tokio::sync::{Mutex, mpsc};
use tokio::time::timeout;
use uuid::Uuid;

use crate::server::RoomCore;
use crate::server::room::RoomHostImpl;
use crate::wasi::bindings;

/// Represents a WebSocket connection to a client.
pub struct WsConnection {
    takeable: Option<TakeableConnection>,
    shared: WsConnectionHandle,
}

#[derive(Clone)]
pub struct WsConnectionHandle {
    pub(crate) id: Uuid,
    pub(crate) auth_data: Option<serde_json::Value>,
    message_rx: Arc<Mutex<Option<mpsc::Receiver<bindings::Message>>>>,
    command_tx: mpsc::Sender<ConnectionCommand>,
}

/// The parts of a WebSocket connection that can only be owned by one task at a time. When a
/// connection si created
struct TakeableConnection {
    command_rx: mpsc::Receiver<ConnectionCommand>,
    message_tx: mpsc::Sender<bindings::Message>,

    /// The raw WebSocket's receiving end. See [`WebSocket::split`] for details.
    raw_ws_rx: SplitStream<WebSocket>,
    /// The raw WebSocket's sending end. See [`WebSocket::split`] for details.
    raw_ws_tx: SplitSink<WebSocket, Message>,
}

pub enum ConnectionCommand {
    Close,
    Send(bindings::Message),
    SendPong(Bytes),
}

impl WsConnection {
    pub async fn init_from_client(
        ws: WebSocket,
        auth_data: Option<serde_json::Value>,
    ) -> anyhow::Result<Self> {
        let (mut raw_ws_tx, mut raw_ws_rx) = ws.split();

        let (message_tx, message_rx) = mpsc::channel::<bindings::Message>(100);
        let (command_tx, command_rx) = mpsc::channel::<ConnectionCommand>(100);

        let id = Uuid::new_v4();

        // Try to receive a handshake message within 1 second of the connection being established.
        // If we can't, or if we receive an invalid message, we'll close the connection.
        //
        // TODO: real error responses/reporting here
        match timeout(Duration::from_secs(1), raw_ws_rx.next()).await {
            Ok(Some(Ok(Message::Text(_message)))) => {
                raw_ws_tx
                    .send(Message::Text(
                        serde_json::to_string(&TxPacket::Handshake::<()>(ServerHandshake { id }))?
                            .into(),
                    ))
                    .await?;

                // TODO: take auth payload and do something
            }
            // Failed to receive a message within the timeout, or an error occurred while receiving
            rest => {
                let mut ws = raw_ws_tx.reunite(raw_ws_rx)?;
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
            shared: WsConnectionHandle {
                id,
                auth_data,
                command_tx: command_tx.clone(),
                message_rx: Arc::new(Mutex::new(Some(message_rx))),
            },
            takeable: Some(TakeableConnection {
                command_rx,
                message_tx,
                raw_ws_rx,
                raw_ws_tx,
            }),
        })
    }

    pub fn handle(&self) -> WsConnectionHandle {
        self.shared.clone()
    }

    /// Called by [`Self::run`] to translate an incoming WebSocket message into a generic "bindings"
    /// message and forwards it to the rest of the system.
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

    /// Processes incoming and outgoing messages for this connection until a close event or command
    /// is received.
    ///
    /// Users should use [`WsConnectionHandle`] to interact with the connection, and should not call
    /// this method directly.
    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut takeable = self
            .takeable
            .take()
            .context("Connection.run() called multiple times")?;

        let commands = self.shared.command_tx.clone();

        loop {
            tokio::select! {
                message = &mut takeable.raw_ws_rx.next() => {
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
                            if let Err(error) = takeable.raw_ws_tx.send(convert_to_axum_message(message)).await {
                                tracing::warn!("an error occurred sending WebSocket message: {error:?}");
                            }
                        },
                        Some(ConnectionCommand::SendPong(frame)) => {
                            if let Err(error) = takeable.raw_ws_tx.send(Message::Pong(frame)).await {
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
impl crate::Connection for WsConnectionHandle {
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

pub fn get_auth_data<R: RoomHostImpl>(
    query_params: &ConnectQueryParams,
    auth_mode: Option<&AuthMode>,
    room: &RoomCore<R>,
) -> Result<Option<serde_json::Value>, ErrorResponse> {
    match &query_params.token {
        Some(token) => {
            // If the mode is JWT, we need to decode and verify the token
            if let Some(AuthMode::Jwt) = auth_mode {
                let decoded = room.decode_token(token).map_err(|e| {
                    ErrorResponse::unauthorized(Some(&format!("invalid token: {}", e)))
                })?;

                Ok(Some(decoded))
            } else {
                // If the auth mode is not JWT, first base64 decode the token and then parse as JSON
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(token)
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

#[derive(Debug)]
pub struct WsUpgradeOptions<R: RoomHostImpl> {
    pub ws: WebSocketUpgrade,
    /// The room to connect this connection to.
    pub room: RoomCore<R>,
    /// The authentication data extracted from the client's request, if any. This is generated by
    /// the user of this API.
    pub auth_data: Option<serde_json::Value>,
    _phantom: std::marker::PhantomData<R>,
}

/// Connects an Axum WebSocket to a room, returning an HTTP response that upgrades the connection.
pub async fn do_ws_upgrade<R: RoomHostImpl>(
    WsUpgradeOptions {
        ws,
        room,
        auth_data,
        ..
    }: WsUpgradeOptions<R>,
) -> Response {
    ws.on_upgrade(|ws| async move {
        tracing::debug!(
            auth = ?auth_data,
            "connecting websocket to room {}", room.id()
        );

        let try_init = |ws: WebSocket, room: RoomCore<R>| async move {
            let connection = WsConnection::init_from_client(ws, auth_data).await?;
            room.add_connection(connection.handle()).await?;
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
