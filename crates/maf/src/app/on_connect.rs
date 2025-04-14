use std::{future::Future, pin::Pin, sync::Arc};

use crate::{App, User};

pub type OnConnectFn = dyn Fn(&App, User) -> Pin<Box<dyn Future<Output = ()>>> + Send + Sync;
type UnitFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub trait IntoOnConnect<Params, Returns> {
    fn into_on_connect(self) -> Arc<OnConnectFn>;
}

pub trait OnConnectParameter: Send {
    fn extract(app: &App, user: &User) -> Self;
}

macro_rules! impl_on_connect_fn {
    ($($members:ident),+) => {
        // non-async impl
        #[allow(unused_parens)]
        impl<
            F: Fn($($members),+) -> () + Clone + Send + Sync + 'static,
            $($members: OnConnectParameter + 'static),+
        > IntoOnConnect<($($members),*), ()> for F {
            fn into_on_connect(self) -> Arc<OnConnectFn> {
                Arc::new(move |app, user| {
                    let f = self.clone();
                    #[allow(non_snake_case)]
                    let ($($members),+) = ($($members::extract(app, &user)),+);
                    Box::pin(async move {
                        f($($members),+);
                    })
                })
            }
        }

        // async impl
        #[allow(unused_parens)]
        impl<
            F: Fn($($members),+) -> Returns + Clone + Send + Sync + 'static,
            Returns: Future<Output = ()> + Send + 'static,
            $($members: OnConnectParameter + 'static),+
        > IntoOnConnect<($($members),*), UnitFuture> for F
        {
            fn into_on_connect(self) -> Arc<OnConnectFn> {
                Arc::new(move |app, user| {
                    let f = self.clone();
                    #[allow(non_snake_case)]
                    let ($($members),+) = ($($members::extract(app, &user)),+);
                    Box::pin(async move {
                        f($($members),+).await;
                    })
                })
            }
        }

    };
}

impl_on_connect_fn!(P1);
impl_on_connect_fn!(P1, P2);
impl_on_connect_fn!(P1, P2, P3);
impl_on_connect_fn!(P1, P2, P3, P4);
impl_on_connect_fn!(P1, P2, P3, P4, P5);
impl_on_connect_fn!(P1, P2, P3, P4, P5, P6);
impl_on_connect_fn!(P1, P2, P3, P4, P5, P6, P7);
impl_on_connect_fn!(P1, P2, P3, P4, P5, P6, P7, P8);

impl OnConnectParameter for User {
    fn extract(_app: &App, user: &User) -> Self {
        user.clone()
    }
}

impl OnConnectParameter for App {
    fn extract(app: &App, _user: &User) -> Self {
        app.clone()
    }
}
