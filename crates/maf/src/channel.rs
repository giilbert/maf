use std::{marker::PhantomData, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};

use crate::{app::AppState, packet::TxPacket, App, User};

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
