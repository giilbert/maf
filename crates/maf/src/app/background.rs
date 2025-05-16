use crate::callable::{AnyCallable, CallableFetch};

use super::App;

pub type BackgroundFn = AnyCallable<BackgroundFnContext, (), BackgroundFnError>;

pub struct BackgroundFnContext {
    pub app: App,
}

#[derive(Debug, thiserror::Error)]
pub enum BackgroundFnError {
    #[error("infalliable error: {0}")]
    Infalliable(#[from] std::convert::Infallible),
}

impl CallableFetch<App> for BackgroundFnContext {
    fn fetch(&self) -> App {
        self.app.clone()
    }
}
