use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::callable::supports::SupportsAsync;
use crate::callable::CallableParam;

/// A boxed callable function that is able to extract parameters from a context and create a future
/// that extracts, runs the wrapped function, and returns the result.
pub type BoxedCallable<Ctx, Ret, Err> =
    Box<dyn Fn(Ctx) -> Pin<Box<dyn Future<Output = Result<Ret, Err>> + Send + Sync>> + Send + Sync>;

/// A trait for converting a function into a boxed callable function. It is useful for converting
/// functions that take different parameters into a common type that can be stored and called.
///
/// ## Usage
/// For a given `Ctx` and `Init`, the function's parameters must all implement
/// [`CallableParam<Ctx, Init>`] for the function to be convertible. The function can be either
/// sync or async, and the return type must be serializable (implement [`serde::Serialize`]). The
/// `Err` type must be able to be constructed from all parameter extraction errors.
///
/// This trait should appear as a trait bound on a generic parameter on a function that registers a
/// callable, e.g. in [`crate::AppBuilder::rpc`]. When using this trait as a trait bound, the
/// `Ctx`, `Err`, and `Init` types should be provided, and `Params`, `Ret`, and `IS_ASYNC` should be
/// taken as type parameters.
pub trait IntoCallable<Ctx, Params, Ret, Err, Init: Send, const IS_ASYNC: bool>:
    Send + Sync + Copy + 'static
{
    fn into_callable(self, init: Init) -> BoxedCallable<Ctx, Ret, Err>;
}

/// Helper macro to implement [`IntoCallable`] for functions with varying numbers of parameters.
macro_rules! impl_into_callable {
    ($($members:ident),*) => {
        #[allow(unused_parens)]
        impl<
            Ctx,
            Ret,
            Err,
            Init,
            $($members),*,
            F
        > IntoCallable<Ctx, ($($members,)*), Ret, Err, Init, false> for F
        where
            F: (Fn($($members),*) -> Ret) + Copy + Send + Sync + 'static,
            $($members: CallableParam<Ctx, Init> + Send + Sync),*,
            $($members::Error: Send + Sync),*,
            $(Err: From<$members::Error>),*,
            // TODO: Replace the trait bound with not just items that can be serialized.
            Ret: serde::Serialize,
            Init: Send + Sync + 'static,
            Ctx: Send + Sync + 'static,
        {
            #[allow(non_snake_case)]
            fn into_callable(self, init: Init) -> BoxedCallable<Ctx, Ret, Err> {
                let init = Arc::new(init);

                Box::new(move |mut ctx| {
                    let init = init.clone();
                    Box::pin(async move {
                        let ($($members),*) = ($($members::extract(&mut ctx, &init).await?),*);
                        Ok(self($($members),*))
                    })
                })
            }
        }

        #[allow(unused_parens)]
        impl<
            Ctx,
            Ret,
            Err,
            Init,
            $($members),*,
            F,
            Fut
        > IntoCallable<Ctx, ($($members,)*), Ret, Err, Init, true> for F
        where
            F: (Fn($($members),*) -> Fut) + Copy + Send + Sync + 'static,
            // Every parameter needs to fit this type of callable and support async tasks.
            $($members: CallableParam<Ctx, Init> + SupportsAsync),*,
            $($members::Error: Send + Sync),*,
            $(Err: From<$members::Error>),*,
            // TODO: Replace the trait bound with not just items that can be serialized.
            Ret: serde::Serialize,
            Init: Send + Sync + 'static,
            Ctx: Send + Sync + 'static,
            Fut: Future<Output = Ret> + Send + Sync + 'static
        {
            #[allow(non_snake_case)]
            fn into_callable(self, init: Init) -> BoxedCallable<Ctx, Ret, Err> {
                let init = Arc::new(init);

                Box::new(move |mut ctx| {
                    let init = init.clone();
                    Box::pin(async move {
                        let ($($members),*) = ($($members::extract(&mut ctx, &init).await?),*);
                        Ok(self($($members),*).await)
                    })
                })
            }
        }
    }
}

// Implementation of IntoCallable for functions with no parameters
impl<Ctx, Ret, Err, Init, F> IntoCallable<Ctx, (), Ret, Err, Init, false> for F
where
    F: (Fn() -> Ret) + Copy + Send + Sync + 'static,
    Init: Send + Sync + 'static,
    Ctx: 'static,
{
    fn into_callable(self, _: Init) -> BoxedCallable<Ctx, Ret, Err> {
        Box::new(move |_| Box::pin(async move { Ok(self()) }))
    }
}

// Implementation of IntoCallable for async functions with no parameters
impl<Ctx, Ret, Err, Init, F, Fut> IntoCallable<Ctx, (), Ret, Err, Init, true> for F
where
    F: (Fn() -> Fut) + Copy + Send + Sync + 'static,
    Init: Send + Sync + 'static,
    Ctx: 'static,
    Fut: Future<Output = Ret> + Send + Sync + 'static,
{
    fn into_callable(self, _: Init) -> BoxedCallable<Ctx, Ret, Err> {
        Box::new(move |_| Box::pin(async move { Ok(self().await) }))
    }
}

// Implementations of `IntoCallable` for functions with 1 to 8 parameters
impl_into_callable!(T1);
impl_into_callable!(T1, T2);
impl_into_callable!(T1, T2, T3);
impl_into_callable!(T1, T2, T3, T4);
impl_into_callable!(T1, T2, T3, T4, T5);
impl_into_callable!(T1, T2, T3, T4, T5, T6);
impl_into_callable!(T1, T2, T3, T4, T5, T6, T7);
impl_into_callable!(T1, T2, T3, T4, T5, T6, T7, T8);
