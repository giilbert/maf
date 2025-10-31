use std::time::Duration;

use anyhow::Context;
use axum::extract::ws::{Message, WebSocket};
use cobble::{platform::Message as PlatformMessage, prelude::Uuid};
use cobble_schemas::packet::{ServerHandshake, TxPacket};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::{Mutex, mpsc},
    time::timeout,
};

pub struct Connection {
    pub(crate) platform: cobble::platform::RawUser,
}

impl Connection {
    pub async fn new(ws: WebSocket) -> anyhow::Result<Self> {
        let id = Uuid::new_v4();
        let (mut ws_tx, mut ws_rx) = ws.split();

        ws_tx
            .send(Message::Text(
                serde_json::to_string(&TxPacket::Handshake::<()>(ServerHandshake { id }))?.into(),
            ))
            .await?;

        // TODO: Refactor this duplicated code (in cobble_container/../container.rs)
        match timeout(Duration::from_secs(1), ws_rx.next()).await {
            Ok(Some(Ok(Message::Text(_message)))) => {
                ws_tx
                    .send(Message::Text(
                        serde_json::to_string(&TxPacket::Handshake::<()>(ServerHandshake { id }))?
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

        let (server_send_tx, mut server_send_rx) = mpsc::channel::<PlatformMessage>(100);
        let (client_send_tx, client_send_rx) = mpsc::channel::<PlatformMessage>(100);

        tokio::spawn(async move {
            let run = move || async move {
                loop {
                    tokio::select! {
                        Some(msg) = ws_rx.next() => {
                            match msg {
                                Ok(msg) => {
                                    if matches!(msg, Message::Text(..) | Message::Binary(..)) {
                                        client_send_tx
                                            .try_send(platform_message_from_ws(msg))
                                            .context("failed to send message to client")?;
                                    }
                                }
                                Err(err) => anyhow::bail!("websocket receive error: {err}"),
                            }
                        }
                        Some(packet) = server_send_rx.recv() => {
                            ws_tx
                                .send(ws_message_from_platform(packet))
                                .await
                                .context("failed to send message to websocket")?;
                        }
                        else => break, // Both channels closed
                    }
                }

                Ok(())
            };

            if let Err(err) = run().await {
                tracing::error!("WebSocket connection error: {err:?}");
            }
        });

        Ok(Self {
            platform: cobble::platform::RawUser {
                messages_tx: server_send_tx,
                messages_rx: Mutex::new(client_send_rx),
                meta: cobble::user::UserMeta { id },
            },
        })
    }
}

/// Converts an axum WebSocket Message into a cobble PlatformMessage, failing if the axum message is an
/// unsupported type (e.g. not text or binary messages). The caller must ensure that only text and
/// binary messages are passed to this function, without everything else being handled elsewhere.
fn platform_message_from_ws(msg: Message) -> PlatformMessage {
    match msg {
        Message::Text(text) => PlatformMessage::Text(text.to_string()),
        Message::Binary(bin) => PlatformMessage::Binary(bin.to_vec()),
        _ => unreachable!("only text and binary messages are handled"),
    }
}

/// Converts a cobble PlatformMessage into an axum WebSocket Message.
fn ws_message_from_platform(msg: PlatformMessage) -> Message {
    match msg {
        PlatformMessage::Text(text) => Message::Text(text.into()),
        PlatformMessage::Binary(bin) => Message::Binary(bin.into()),
    }
}
