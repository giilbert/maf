use std::{marker::PhantomData, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::broadcast;

use crate::{
    app::AppState,
    packet::{ChannelSendRx, TxPacket},
    platform::SendError,
    User,
};

#[derive(Debug)]
pub struct Channel<T> {
    name: String,
    state: Arc<AppState>,
    rx: Option<broadcast::Receiver<ChannelSendRx>>,
    _phantom: PhantomData<T>,
}

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error("failed to receive message {0}")]
    Recv(#[from] broadcast::error::RecvError),
    #[error("failed to deserialize message")]
    Deserialize(#[from] serde_json::Error),
}

impl<T> Channel<T> {
    pub fn new(state: Arc<AppState>, name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            state,
            rx: None,
            _phantom: PhantomData,
        }
    }
}

impl<T: Serialize> Channel<T> {
    pub fn send(&self, user: &User, message: T) -> Result<(), SendError> {
        user.send(TxPacket::ChannelSend {
            channel: &self.name,
            data: &message,
        })?;

        Ok(())
    }

    pub async fn broadcast(&self, message: T) -> Result<(), SendError> {
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
    async fn lazy_init_recv(
        &mut self,
        user: Option<&User>,
    ) -> &mut broadcast::Receiver<ChannelSendRx> {
        if self.rx.is_some() {
            return self.rx.as_mut().expect("rx is None");
        } else {
            // Create the broadcast channel if it doesn't exist
            match user {
                // This channel is bound to a specific user, use a different method to rx messages
                Some(user) => {
                    let user_id = user.meta.id;

                    if !self
                        .state
                        .user_rx_channels
                        .read()
                        .await
                        .contains_key(&(user_id, self.name.clone()))
                    {
                        self.state.user_rx_channels.write().await.insert(
                            (user_id, self.name.clone()),
                            UntypedChannelBroadcast::default(),
                        );
                    }

                    let mut channels = self.state.user_rx_channels.write().await;
                    let channel = channels
                        .get_mut(&(user_id, self.name.clone()))
                        .expect("channel not found");
                    self.rx = Some(channel.tx.subscribe());
                }
                None => {
                    if !self.state.channels.read().await.contains_key(&self.name) {
                        self.state
                            .channels
                            .write()
                            .await
                            .insert(self.name.clone(), UntypedChannelBroadcast::default());
                    }

                    let mut channels = self.state.channels.write().await;
                    let channel = channels.get_mut(&self.name).expect("channel not found");
                    self.rx = Some(channel.tx.subscribe());
                }
            }

            return self.rx.as_mut().expect("rx is None");
        }
    }

    pub async fn recv(&mut self) -> Result<T, RecvError> {
        let message = self.lazy_init_recv(None).await.recv().await?;
        let data = serde_json::from_value(message.data)?;
        Ok(data)
    }

    pub async fn recv_user(&mut self, user: &User) -> Result<T, RecvError> {
        let message = self.lazy_init_recv(Some(user)).await.recv().await?;
        let data = serde_json::from_value(message.data)?;
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
    pub fn send(&self, message: T) -> Result<(), SendError> {
        self.channel.send(&self.user, message)
    }
}

impl<T: DeserializeOwned> BoundChannel<T> {
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        self.channel.recv_user(&self.user).await
    }
}

#[derive(Debug, Clone)]
pub struct UntypedChannelBroadcast {
    pub(crate) tx: broadcast::Sender<ChannelSendRx>,
}

impl Default for UntypedChannelBroadcast {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel(20);
        Self { tx }
    }
}
