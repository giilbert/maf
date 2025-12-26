use uuid::Uuid;

use crate::{
    app::{
        hooks::{self, HookBody, HookRequestError, HookRequestInit},
        meta,
    },
    bindings::bindgen,
    platform::{ListenError, Platform, PlatformHookRequest, PlatformUser, SendError},
    tasks,
    user::{UserMeta, UserNextMessageError},
};

pub struct WasiPlatform {
    future_user: bindgen::FutureUser,
    future_hook_request: bindgen::FutureHookRequest,
}

impl Platform for WasiPlatform {
    type Config = ();

    fn init(_config: Self::Config) -> anyhow::Result<Self> {
        Ok(Self {
            future_user: bindgen::listen_user()?,
            future_hook_request: bindgen::listen_hook_request()?,
        })
    }

    async fn next_user(&self) -> Result<RawUser, ListenError> {
        let pollable = self.future_user.subscribe()?;
        tasks::wait_for(pollable).await;
        Ok(RawUser::new(self.future_user.get()?))
    }

    async fn next_hook_request(&self) -> Result<RawHookRequest, ListenError> {
        let pollable = self.future_hook_request.subscribe()?;
        tasks::wait_for(pollable).await;
        Ok(RawHookRequest::new(self.future_hook_request.get()?))
    }

    fn report_app_schema(&self, schema: &str) {
        bindgen::report_app_schema(schema);
    }

    fn set_meta(
        &self,
        visibility: meta::MetaVisibility,
        key: &str,
        value: &str,
    ) -> Option<meta::MetaEntry> {
        bindgen::set_meta(visibility.into(), key, value).map(|entry| entry.into())
    }

    fn get_meta(&self, key: &str) -> Option<meta::MetaEntry> {
        bindgen::get_meta(key).map(|entry| entry.into())
    }

    fn delete_meta(&self, key: &str) -> Option<meta::MetaEntry> {
        bindgen::delete_meta(key).map(|entry| entry.into())
    }

    fn list_meta(&self) -> Vec<(String, meta::MetaEntry)> {
        bindgen::list_meta()
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect()
    }
}

#[derive(Debug)]
pub struct RawUser {
    inner: bindgen::User,
    messages: bindgen::FutureMessage,
}

impl RawUser {
    pub fn new(raw: bindgen::User) -> Self {
        Self {
            messages: raw.listen_message().expect("Failed to listen for messages"),
            inner: raw,
        }
    }
}

impl PlatformUser for RawUser {
    fn meta(&self) -> UserMeta {
        let bindings_meta = self.inner.meta();

        UserMeta {
            id: Uuid::from_u64_pair(bindings_meta.id.0, bindings_meta.id.1),
        }
    }

    fn send(&self, message: super::Message) -> Result<(), super::SendError> {
        self.inner.send(&message.into()).map_err(SendError::from)
    }

    async fn next_message(&self) -> Result<super::Message, UserNextMessageError> {
        tasks::wait_for(
            self.messages
                .subscribe()
                .map_err(|e| UserNextMessageError::Listen(e.into()))?,
        )
        .await;

        let message = self
            .messages
            .get()
            .map_err(|e| UserNextMessageError::Listen(e.into()))?;

        Ok(super::Message::from(message))
    }
}

#[derive(Debug)]
pub struct RawHookRequest {
    inner: bindgen::HookRequest,
}

impl RawHookRequest {
    pub fn new(raw: bindgen::HookRequest) -> Self {
        Self { inner: raw }
    }
}

impl PlatformHookRequest for RawHookRequest {
    fn init(&self) -> Result<HookRequestInit, HookRequestError> {
        self.inner
            .init()
            .map(|init| HookRequestInit {
                caller: init.caller.into(),
                method: init.method,
                data: init.data.into(),
            })
            .map_err(Into::into)
    }

    fn respond(&self, body: HookBody) -> Result<(), SendError> {
        self.inner.respond(&body.into()).map_err(SendError::from)
    }
}

/// Implementations for converting between WIT bindings and platform types.
mod conversion_impls {
    use super::*;
    use crate::platform;

    impl From<bindgen::Message> for platform::Message {
        fn from(value: bindgen::Message) -> Self {
            match value {
                bindgen::Message::Text(text) => platform::Message::Text(text),
                bindgen::Message::Binary(data) => platform::Message::Binary(data),
            }
        }
    }

    impl From<platform::Message> for bindgen::Message {
        fn from(value: platform::Message) -> Self {
            match value {
                platform::Message::Text(text) => bindgen::Message::Text(text),
                platform::Message::Binary(data) => bindgen::Message::Binary(data),
            }
        }
    }

    impl From<bindgen::SendError> for platform::SendError {
        fn from(value: bindgen::SendError) -> Self {
            match value {
                bindgen::SendError::Closed => super::SendError::Closed,
                bindgen::SendError::BufferFull => super::SendError::BufferFull,
            }
        }
    }

    impl From<bindgen::ListenError> for platform::ListenError {
        fn from(value: bindgen::ListenError) -> Self {
            match value {
                bindgen::ListenError::Closed => super::ListenError::Closed,
                bindgen::ListenError::NotReady => super::ListenError::NotReady,
                bindgen::ListenError::AlreadyListening => super::ListenError::AlreadyListening,
            }
        }
    }

    impl From<bindgen::HookBody> for HookBody {
        fn from(value: bindgen::HookBody) -> Self {
            match value {
                bindgen::HookBody::Json(text) => HookBody::Json(text),
                bindgen::HookBody::None => HookBody::None,
            }
        }
    }

    impl From<HookBody> for bindgen::HookBody {
        fn from(value: HookBody) -> Self {
            match value {
                HookBody::Json(text) => bindgen::HookBody::Json(text),
                HookBody::None => bindgen::HookBody::None,
            }
        }
    }

    impl From<bindgen::HookRequestCaller> for hooks::HookRequestCaller {
        fn from(value: bindgen::HookRequestCaller) -> Self {
            match value {
                bindgen::HookRequestCaller::Service => hooks::HookRequestCaller::Service,
            }
        }
    }

    impl From<bindgen::HookRequestError> for hooks::HookRequestError {
        fn from(value: bindgen::HookRequestError) -> Self {
            match value {
                bindgen::HookRequestError::InitConsumed => hooks::HookRequestError::InitConsumed,
            }
        }
    }

    impl From<bindgen::MetaVisibility> for meta::MetaVisibility {
        fn from(value: bindgen::MetaVisibility) -> Self {
            match value {
                bindgen::MetaVisibility::Public => meta::MetaVisibility::Public,
                bindgen::MetaVisibility::Private => meta::MetaVisibility::Private,
            }
        }
    }

    impl From<meta::MetaVisibility> for bindgen::MetaVisibility {
        fn from(value: meta::MetaVisibility) -> Self {
            match value {
                meta::MetaVisibility::Public => bindgen::MetaVisibility::Public,
                meta::MetaVisibility::Private => bindgen::MetaVisibility::Private,
            }
        }
    }

    impl From<bindgen::MetaEntry> for meta::MetaEntry {
        fn from(value: bindgen::MetaEntry) -> Self {
            meta::MetaEntry {
                visibility: value.visibility.into(),
                value: value.value,
            }
        }
    }

    impl From<meta::MetaEntry> for bindgen::MetaEntry {
        fn from(value: meta::MetaEntry) -> Self {
            bindgen::MetaEntry {
                visibility: value.visibility.into(),
                value: value.value,
            }
        }
    }
}
