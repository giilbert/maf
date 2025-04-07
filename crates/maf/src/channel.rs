use std::{marker::PhantomData, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};

use crate::{app::AppState, packet::TxPacket, App, User};

#[derive(Debug)]
pub struct Channel<T: Serialize + DeserializeOwned> {
    name: String,
    state: Arc<AppState>,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> Channel<T> {
    pub fn new(app: &App, name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            state: app.state.clone(),
            _phantom: PhantomData,
        }
    }

    pub async fn send(&self, user: &User, message: T) -> anyhow::Result<()> {
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
