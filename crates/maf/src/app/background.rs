use std::{future::Future, pin::Pin, sync::Arc};

use crate::utils::UnitFuture;

use super::App;

pub type BackgroundFn = dyn Fn(App) -> Pin<Box<dyn Future<Output = ()>>> + Send + Sync;

pub trait IntoBackgroundFn<T> {
    fn into_background_fn(self) -> Arc<BackgroundFn>;
}

impl<F, R> IntoBackgroundFn<UnitFuture> for F
where
    R: Future<Output = ()> + Send + 'static,
    F: Fn(App) -> R + Send + Sync + 'static,
{
    fn into_background_fn(self) -> Arc<BackgroundFn> {
        Arc::new(move |app| {
            let f = self(app);
            Box::pin(f)
        })
    }
}

impl<F> IntoBackgroundFn<()> for F
where
    F: Fn(App) -> () + Send + Sync + 'static,
{
    fn into_background_fn(self) -> Arc<BackgroundFn> {
        Arc::new(move |app| {
            self(app);
            Box::pin(async {})
        })
    }
}
