use std::{any::Any, marker::PhantomData, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::broadcast;

use crate::{
    app::AppState,
    bindings::bindgen,
    packet::{ChannelSendRx, TxPacket},
    App, User,
};

#[derive(Debug)]
pub struct Channel<T> {
    name: String,
    state: Arc<AppState>,
    _phantom: PhantomData<T>,
}

impl<T> Channel<T> {
    pub fn new(state: Arc<AppState>, name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            state,
            _phantom: PhantomData,
        }
    }
}

impl<T: Serialize> Channel<T> {
    pub fn send(&self, user: &User, message: T) -> anyhow::Result<()> {
        user.send(TxPacket::ChannelSend {
            channel: &self.name,
            data: &message,
        })?;

        Ok(())
    }

    pub async fn broadcast(&self, message: T) -> anyhow::Result<()> {
        let users = self.state.users.read().await;
        for user in users.values() {
            user.send(TxPacket::ChannelSend {
                channel: &self.name,
                data: &message,
            })?;
        }

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T: DeserializeOwned> Channel<T> {
    pub async fn recv(&self) -> anyhow::Result<T> {
        if self.state.channels.read().await.get(&self.name).is_none() {
            self.state
                .channels
                .write()
                .await
                .insert(self.name.clone(), UntypedChannelBroadcast::default());
        }
        let message = self
            .state
            .channels
            .read()
            .await
            .get(&self.name)
            .expect("channel not found")
            .tx
            .subscribe()
            .recv()
            .await?;

        let data = serde_json::from_value(message.data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize message: {}", e))?;

        Ok(data)
    }
}

pub struct BoundChannel<T> {
    channel: Channel<T>,
    user: User,
}

impl<T> BoundChannel<T> {
    pub fn new(channel: Channel<T>, user: &User) -> Self {
        Self {
            channel,
            user: user.clone(),
        }
    }
}

impl<T: Serialize> BoundChannel<T> {
    pub fn send(&self, message: T) -> anyhow::Result<()> {
        self.channel.send(&self.user, message)
    }
}

impl<T: DeserializeOwned> BoundChannel<T> {
    // TODO:
    pub async fn recv(&self) -> anyhow::Result<T> {
        todo!("recv for channels bound to users");
    }
}

#[derive(Debug)]
pub struct UntypedChannelBroadcast {
    pub(crate) tx: broadcast::Sender<ChannelSendRx>,
}

impl Default for UntypedChannelBroadcast {
    fn default() -> Self {
        let (tx, rx) = broadcast::channel(20);
        Self { tx }
    }
}
