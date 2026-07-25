use super::App;
use super::local::LocalStateError;
use crate::callable::{BoxedCallable, CallableFetch};

pub type BackgroundFn = BoxedCallable<BackgroundFnContext, (), BackgroundFnError>;

#[derive(Clone)]
pub struct BackgroundFnContext {
    pub app: App,
}

#[derive(Debug, thiserror::Error)]
pub enum BackgroundFnError {
    #[error("state error: {0}")]
    State(#[from] LocalStateError),

    #[error("infalliable error: {0}")]
    Infalliable(#[from] std::convert::Infallible),
}

impl CallableFetch<App> for BackgroundFnContext {
    fn fetch(&self) -> App {
        self.app.clone()
    }
}
