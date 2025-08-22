use tokio::sync::{
    mpsc::{self, error::TrySendError},
    Mutex,
};

use crate::{
    platform::{ListenError, Message, Platform, PlatformHookRequest, PlatformUser},
    user::UserMeta,
};

/// Platforms that are implemented with the actor model: abstractions are implemented by sending
/// messages through channels.
pub struct ActorPlatform {
    users_rx: Mutex<mpsc::Receiver<RawUser>>,
    hook_request_rx: Mutex<mpsc::Receiver<RawHookRequest>>,

    // NOTE: Use methods on ActorPlatform to user these channels
    users_tx: mpsc::Sender<RawUser>,
    hook_request_tx: mpsc::Sender<RawHookRequest>,
}

impl ActorPlatform {}

impl Platform for ActorPlatform {
    type Config = ();

    fn init(_config: Self::Config) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (users_tx, users_rx) = mpsc::channel(100);
        let (hook_request_tx, hook_request_rx) = mpsc::channel(100);

        Ok(Self {
            users_rx: Mutex::new(users_rx),
            users_tx,
            hook_request_rx: Mutex::new(hook_request_rx),
            hook_request_tx,
        })
    }

    async fn next_user(&self) -> Result<super::RawUser, super::ListenError> {
        self.users_rx
            .try_lock()
            .map_err(|_| super::ListenError::AlreadyListening)?
            .recv()
            .await
            .ok_or(super::ListenError::Closed)
    }

    async fn next_hook_request(&self) -> Result<super::RawHookRequest, super::ListenError> {
        self.hook_request_rx
            .try_lock()
            .map_err(|_| super::ListenError::AlreadyListening)?
            .recv()
            .await
            .ok_or(super::ListenError::Closed)
    }

    fn report_app_schema(&self, schema: &str) {
        println!("TODO: report_app_schema with {schema}");
    }
}

pub struct RawUser {
    pub meta: UserMeta,
    // Client to server messages
    pub messages_rx: Mutex<mpsc::Receiver<Message>>,
    // Server to client messages
    pub messages_tx: mpsc::Sender<Message>,
}

impl PlatformUser for RawUser {
    fn meta(&self) -> UserMeta {
        self.meta.clone()
    }

    fn send(&self, message: super::Message) -> Result<(), super::SendError> {
        self.messages_tx.try_send(message).map_err(|e| match e {
            TrySendError::Full(_) => super::SendError::BufferFull,
            TrySendError::Closed(_) => super::SendError::Closed,
        })
    }

    async fn next_message(&self) -> Result<super::Message, crate::user::UserNextMessageError> {
        self.messages_rx
            .try_lock()
            .map_err(|_| crate::user::UserNextMessageError::Listen(ListenError::AlreadyListening))?
            .recv()
            .await
            .ok_or_else(|| crate::user::UserNextMessageError::Listen(ListenError::Closed))
    }
}

pub struct RawHookRequest {}

impl PlatformHookRequest for RawHookRequest {
    fn init(
        &self,
    ) -> Result<crate::app::hooks::HookRequestInit, crate::app::hooks::HookRequestError> {
        todo!()
    }

    fn respond(&self, body: crate::app::hooks::HookBody) -> Result<(), super::SendError> {
        todo!()
    }
}
