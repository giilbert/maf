use std::{any::TypeId, collections::HashMap, sync::Arc};

use crate::{
    callable::{BoxedCallable, CallableFetch},
    platform::{self, PlatformHookRequest},
};

use super::{App, AppState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookRequestCaller {
    Service,
}

#[derive(Debug)]
pub enum HookBody {
    Json(String),
    None,
}

#[allow(unused)]
pub struct HookRequest {
    pub caller: HookRequestCaller,
    pub method: String,
    pub data: HookBody,
    state: Arc<AppState>,
    raw: platform::RawHookRequest,
}

#[derive(Debug, thiserror::Error)]
pub enum HookRequestError {
    #[error("hook request init already consumed")]
    InitConsumed,
}

impl HookRequest {
    pub fn new(
        state: Arc<AppState>,
        raw: platform::RawHookRequest,
    ) -> Result<Self, HookRequestError> {
        let init = raw.init()?;
        Ok(Self {
            caller: init.caller,
            data: init.data,
            method: init.method,
            state,
            raw,
        })
    }

    pub fn respond(&self, body: HookBody) -> Result<(), platform::SendError> {
        self.raw.respond(body)
    }
}

pub struct HookContext {
    pub app: App,
    pub request: Arc<HookRequest>,
}

pub struct HookResponse {
    pub body: HookBody,
}

pub struct HookRequestInit {
    pub caller: HookRequestCaller,
    pub method: String,
    pub data: HookBody,
}

#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("hook method `{0}` not found")]
    MethodNotFound(String),
    #[error("hook function in `{0}` error: {1}")]
    FunctionError(String, anyhow::Error),
    #[error("hook response serialization error: {0}")]
    ResponseSerializationError(#[from] serde_json::Error),
    #[error("hook response error: {0}")]
    ResponseError(#[from] platform::SendError),
    #[error("infalliable error: {0}")]
    Infalliable(#[from] std::convert::Infallible),
}

#[allow(unused)]
pub struct HookFunction {
    pub type_id: TypeId,
    pub method: String,
    pub callable: BoxedCallable<HookContext, HookResponse, HookError>,
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
        request: HookRequest,
    ) -> Result<(), HookError> {
        let method = request.method.clone();
        if let Some(hook) = self.hooks.get(&method) {
            let request = Arc::new(request);
            let context = HookContext {
                app,
                request: request.clone(),
            };

            let response = (hook.callable)(context)
                .await
                .map_err(|err| HookError::FunctionError(method.clone(), anyhow::anyhow!(err)))?;

            request.respond(response.body)?;

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
