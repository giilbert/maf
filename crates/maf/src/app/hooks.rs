use std::{
    any::TypeId,
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::{
    bindings::bindgen,
    callable::{AnyCallable, CallableFetch},
    tasks::Runtime,
};

use super::{App, AppState};

pub struct HookRequest {
    pub caller: bindgen::HookRequestCaller,
    pub method: String,
    pub data: Option<bindgen::HookBody>,
    state: Arc<AppState>,
    raw: Option<bindgen::HookRequest>,
}

pub struct HooksListener {
    state: Arc<AppState>,
    future_hook_request: bindgen::FutureHookRequest,
}

impl HooksListener {
    pub fn new(state: Arc<AppState>) -> Result<Self, bindgen::ListenError> {
        Ok(Self {
            state,
            future_hook_request: bindgen::listen_hook_request()?,
        })
    }

    pub fn next(&self) -> HooksListenerNextFuture<'_> {
        HooksListenerNextFuture {
            state: &self.state,
            listener: &self.future_hook_request,
        }
    }
}

pub struct HooksListenerNextFuture<'a> {
    state: &'a Arc<AppState>,
    listener: &'a bindgen::FutureHookRequest,
}

impl HooksListenerNextFuture<'_> {
    pub fn try_poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Result<Poll<HookRequest>, bindgen::ListenError> {
        match self.listener.get() {
            Ok(raw_user) => {
                return Ok(Poll::Ready(
                    HookRequest::new(self.state.clone(), raw_user)
                        .map_err(|_| bindgen::ListenError::NotReady)?,
                ))
            }
            Err(bindgen::ListenError::NotReady) => {
                let pollable = self.listener.subscribe()?;
                if pollable.ready() {
                    return self.try_poll(cx);
                } else {
                    Runtime::new_waker(cx, pollable, Some("listen user"));
                    Ok(Poll::Pending)
                }
            }
            Err(err) => return Err(err),
        }
    }
}

impl Future for HooksListenerNextFuture<'_> {
    type Output = Result<HookRequest, bindgen::ListenError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.try_poll(cx) {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

impl HookRequest {
    pub fn new(
        state: Arc<AppState>,
        raw: bindgen::HookRequest,
    ) -> Result<Self, bindgen::HookRequestError> {
        let init = raw.init()?;
        Ok(Self {
            state,
            caller: init.caller,
            method: init.method,
            data: Some(init.data),
            raw: Some(raw),
        })
    }

    fn raw(&mut self) -> bindgen::HookRequest {
        self.raw.take().expect("raw request already taken")
    }
}

pub struct HookContext {
    pub app: App,
    pub request: HookRequest,
}

pub struct HookResponse {
    pub body: bindgen::HookBody,
}

pub struct HookInit {}

#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("hook method `{0}` not found")]
    MethodNotFound(String),
    #[error("hook function in `{0}` error: {1}")]
    FunctionError(String, anyhow::Error),
    #[error("hook response serialization error: {0}")]
    ResponseSerializationError(#[from] serde_json::Error),
    #[error("hook response error: {0}")]
    ResponseError(#[from] bindgen::SendError),
    #[error("infalliable error: {0}")]
    Infalliable(#[from] std::convert::Infallible),
}

pub struct HookFunction {
    pub type_id: TypeId,
    pub method: String,
    pub callable: AnyCallable<HookContext, HookResponse, HookError>,
}

#[derive(Default)]
pub struct HookStore {
    hooks: HashMap<String, HookFunction>,
}

impl HookStore {
    pub fn add_hook_function(&mut self, hook: HookFunction) {
        self.hooks.insert(hook.method.clone(), hook);
    }

    pub async fn handle_hook_request(
        &self,
        app: App,
        mut request: HookRequest,
    ) -> Result<(), HookError> {
        let method = request.method.clone();
        if let Some(hook) = self.hooks.get(&method) {
            let raw = request.raw();

            let context = HookContext { app, request };

            let response = (hook.callable)(context)
                .await
                .map_err(|err| HookError::FunctionError(method.clone(), anyhow::anyhow!(err)))?;

            raw.respond(&response.body)?;

            return Ok(());
        }

        Err(HookError::MethodNotFound(method))
    }
}

impl CallableFetch<App> for HookContext {
    fn fetch(&self) -> App {
        self.app.clone()
    }
}
