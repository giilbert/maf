//! Primitive for sending and receiving messages between the server and the client.

use std::{marker::PhantomData, sync::Arc};

use cobble_schemas::packet::{ChannelSendRx, TxPacket};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::broadcast;

use crate::{app::AppState, platform::SendError, User};

/// A named channel that can be used to send and receive messages of type `T` between the server and
/// the client.
///
/// There is no guarantee that messages will be received in the order they were sent and there is no
/// guarantee that messages will be received at all.
///
/// To get a channel, acquire an [`crate::App`] and call [`crate::App::channel`] with the channel's
/// name and the type of data being sent through the channel as a type parameter.
///
/// ## Example
///
/// ```rust
/// App::builder()
///     .background(|app: App| async move {
///         let channel = app.channel::<String>("messages");
///         loop {
///             channel.broadcast("Hello, world!".to_string()).await.ok();
///             tasks::sleep(std::time::Duration::from_secs(5)).await;
///         }
///     })
/// ```
///
/// ## Bound Channels
/// A [`Channel`] can be bound to a specific user using [`BoundChannel::new`]. This allows for
/// sending/receiving messages that are specific to that user.
#[derive(Debug)]
pub struct Channel<T> {
    name: String,
    rx: Option<broadcast::Receiver<ChannelSendRx>>,
    state: Arc<AppState>,
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

/// Half implementation of channel functionality for sending messages that are serializable.
impl<T: Serialize> Channel<T> {
    /// Sends a message to a single user.
    ///
    /// There is no guarantee that the user will receive the message and messages sent may be
    /// processed out of order.
    pub fn send(&self, user: &User, message: T) -> Result<(), SendError> {
        user.send(TxPacket::ChannelSend {
            channel: &self.name,
            data: &message,
        })?;

        Ok(())
    }

    /// Sends a message to all connected users.
    ///
    /// There is no guarantee that users will receive the message and messages sent may be
    /// processed out of order.
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

    /// Returns the name of the channel.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T: DeserializeOwned> Channel<T> {
    /// Gets the underlying broadcast receiver, creating it based on a strategy if it doesn't exist.
    async fn lazy_get_recv(
        &mut self,
        user: Option<&User>,
    ) -> &mut broadcast::Receiver<ChannelSendRx> {
        if self.rx.is_some() {
            return self.rx.as_mut().expect("rx is None");
        } else {
            // Create the broadcast channel if it doesn't exist
            match user {
                // If this channel is bounded to a user, create a user-specific channel and
                // register it to app state if it doesn't exist
                Some(user) => {
                    let user_id = user.meta().id;

                    let does_user_channel_exist = self
                        .state
                        .user_rx_channels
                        .read()
                        .await
                        .contains_key(&(user_id, self.name.clone()));

                    // Create the user-specific channel if it doesn't exist
                    if !does_user_channel_exist {
                        self.state.user_rx_channels.write().await.insert(
                            (user_id, self.name.clone()),
                            UntypedChannelBroadcast::default(),
                        );
                    }

                    let mut channels = self.state.user_rx_channels.write().await;
                    let channel = channels
                        .get_mut(&(user_id, self.name.clone()))
                        .expect("channel should exist");

                    self.rx = Some(channel.tx.subscribe());
                }
                // If this channel is not bounded to a user, create a global channel and register it
                // to app state if it doesn't exist
                None => {
                    let does_channel_exist =
                        self.state.channels.read().await.contains_key(&self.name);

                    if !does_channel_exist {
                        self.state
                            .channels
                            .write()
                            .await
                            .insert(self.name.clone(), UntypedChannelBroadcast::default());
                    }

                    let mut channels = self.state.channels.write().await;
                    let channel = channels.get_mut(&self.name).expect("channel should exist");

                    self.rx = Some(channel.tx.subscribe());
                }
            }

            return self.rx.as_mut().expect("rx is None");
        }
    }

    pub async fn recv(&mut self) -> Result<T, RecvError> {
        let message = self.lazy_get_recv(None).await.recv().await?;
        let data = serde_json::from_value(message.data)?;
        Ok(data)
    }

    pub async fn recv_user(&mut self, user: &User) -> Result<T, RecvError> {
        let message = self.lazy_get_recv(Some(user)).await.recv().await?;
        let data = serde_json::from_value(message.data)?;
        Ok(data)
    }
}

/// A channel that is bound to a specific user. [`BoundChannel`] will only send and receive messages
/// that are specific to that user.
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

    /// Returns a reference to the user this channel is bound to.
    pub fn user(&self) -> &User {
        &self.user
    }

    /// Returns a reference to the underlying channel.
    pub fn channel(&self) -> &Channel<T> {
        &self.channel
    }
}

impl<T: Serialize> BoundChannel<T> {
    /// Sends a message to the user this channel is bound to. For more details, see
    /// [`Channel::send`].
    pub fn send(&self, message: T) -> Result<(), SendError> {
        self.channel.send(&self.user, message)
    }
}

impl<T: DeserializeOwned> BoundChannel<T> {
    /// Receives a message from the user this channel is bound to. For more details, see
    /// [`Channel::recv_user`].
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        self.channel.recv_user(&self.user).await
    }
}

/// Used internally to store broadcast channels in app state without needing to know the type.
#[derive(Debug, Clone)]
pub(crate) struct UntypedChannelBroadcast {
    pub(crate) tx: broadcast::Sender<ChannelSendRx>,
}

const MAX_CHANNEL_BUFFER: usize = 20;

impl Default for UntypedChannelBroadcast {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel(MAX_CHANNEL_BUFFER);
        Self { tx }
    }
}
