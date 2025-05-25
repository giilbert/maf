use std::{future::Future, marker::PhantomData, pin::Pin, sync::Arc};

use crate::callable::CallableParam;

pub type AnyCallable<Ctx, Ret, Err> =
    Box<dyn Fn(Ctx) -> Pin<Box<dyn Future<Output = Result<Ret, Err>>>> + Send + Sync>;

pub trait IntoCallable<Ctx, Params, Ret, Err, Init: Send, const IS_ASYNC: bool>:
    Send + Sync + Copy + 'static
{
    fn into_callable(self, init: Init) -> AnyCallable<Ctx, Ret, Err>;
}

macro_rules! impl_into_callable {
    ($($members:ident),+) => {
        #[allow(unused_parens)]
        impl<
            Ctx,
            Ret,
            Err,
            Init,
            $($members),*,
            F
        > IntoCallable<Ctx, ($($members),*), Ret, Err, Init, false> for F
        where
            F: (Fn($($members),*) -> Ret) + Copy + Send + Sync + 'static,
            $($members: CallableParam<Ctx, Init>),*,
            $(Err: From<$members::Error>),*,
            Ret: serde::Serialize,
            Init: Send + Sync + 'static,
            Ctx: 'static,
        {
            #[allow(non_snake_case)]
            fn into_callable(self, init: Init) -> AnyCallable<Ctx, Ret, Err> {
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
        > IntoCallable<Ctx, PhantomData<($($members),*)>, Ret, Err, Init, false> for F
        where
            F: (Fn($($members),*) -> Fut) + Copy + Send + Sync + 'static,
            $($members: CallableParam<Ctx, Init>),*,
            $(Err: From<$members::Error>),*,
            Ret: serde::Serialize,
            Init: Send + Sync + 'static,
            Ctx: 'static,
            Fut: Future<Output = Ret> + Send + Sync + 'static
        {
            #[allow(non_snake_case)]
            fn into_callable(self, init: Init) -> AnyCallable<Ctx, Ret, Err> {
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

impl<Ctx, Ret, Err, Init, F> IntoCallable<Ctx, (), Ret, Err, Init, false> for F
where
    F: (Fn() -> Ret) + Copy + Send + Sync + 'static,
    Init: Send + Sync + 'static,
    Ctx: 'static,
{
    fn into_callable(self, _: Init) -> AnyCallable<Ctx, Ret, Err> {
        Box::new(move |_| Box::pin(async move { Ok(self()) }))
    }
}

impl<Ctx, Ret, Err, Init, F, Fut> IntoCallable<Ctx, (), Ret, Err, Init, true> for F
where
    F: (Fn() -> Fut) + Copy + Send + Sync + 'static,
    Init: Send + Sync + 'static,
    Ctx: 'static,
    Fut: Future<Output = Ret> + Send + Sync + 'static,
{
    fn into_callable(self, _: Init) -> AnyCallable<Ctx, Ret, Err> {
        Box::new(move |_| Box::pin(async move { Ok(self().await) }))
    }
}

impl<Ctx, Ret, Err, Init, T1, F> IntoCallable<Ctx, (T1,), Ret, Err, Init, false> for F
where
    F: (Fn(T1) -> Ret) + Copy + Send + Sync + 'static,
    T1: CallableParam<Ctx, Init>,
    Err: From<T1::Error>,
    Ret: serde::Serialize,
    Init: Send + Sync + 'static,
    Ctx: 'static,
    Err: From<T1::Error>,
{
    #[allow(non_snake_case)]
    fn into_callable(self, init: Init) -> AnyCallable<Ctx, Ret, Err> {
        let init = Arc::new(init);
        Box::new(move |mut ctx| {
            let init = init.clone();
            Box::pin(async move {
                let (T1,) = (T1::extract(&mut ctx, &init).await?,);
                Ok(self(T1))
            })
        })
    }
}

impl<Ctx, Ret, Err, Init, T1, F, Fut> IntoCallable<Ctx, (T1,), Ret, Err, Init, true> for F
where
    F: (Fn(T1) -> Fut) + Copy + Send + Sync + 'static,
    T1: CallableParam<Ctx, Init>,
    Err: From<T1::Error>,
    Ret: serde::Serialize,
    Init: Send + Sync + 'static,
    Ctx: 'static,
    Fut: Future<Output = Ret> + Send + Sync + 'static,
{
    #[allow(non_snake_case)]
    fn into_callable(self, init: Init) -> AnyCallable<Ctx, Ret, Err> {
        let init = Arc::new(init);
        Box::new(move |mut ctx| {
            let init = init.clone();
            Box::pin(async move {
                let (T1,) = (T1::extract(&mut ctx, &init).await?,);
                Ok(self(T1).await)
            })
        })
    }
}

impl_into_callable!(T1, T2);
impl_into_callable!(T1, T2, T3);
impl_into_callable!(T1, T2, T3, T4);
impl_into_callable!(T1, T2, T3, T4, T5);
impl_into_callable!(T1, T2, T3, T4, T5, T6);
impl_into_callable!(T1, T2, T3, T4, T5, T6, T7);
impl_into_callable!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_into_callable!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
