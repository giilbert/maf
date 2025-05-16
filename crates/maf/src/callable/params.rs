use crate::{App, User};

/// A parameter that could appear in a callable function (e.g. `rpc` or `on_connect`).
///
/// This trait is used to extract the parameter from the context and/or initialize it from
/// parameters used to create the callable function.
pub trait CallableParam<Ctx, Init>: Sized {
    type Error: std::error::Error + Send;

    fn extract(
        ctx: &mut Ctx,
        init: &Init,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> + Send;
}

/// A trait for simple types that can be used as parameters in callable functions.
pub trait CallableFetch<T>: Send {
    fn fetch(&self) -> T;
}

// Implement `CallableParams` for common types
macro_rules! impl_callable_param_fetch {
    ($t:ty) => {
        impl<Ctx: CallableFetch<$t>, Init: Send + Sync> CallableParam<Ctx, Init> for $t {
            type Error = std::convert::Infallible;

            async fn extract(ctx: &mut Ctx, _init: &Init) -> Result<Self, Self::Error> {
                Ok(ctx.fetch())
            }
        }
    };
}

impl_callable_param_fetch!(User);
impl_callable_param_fetch!(App);
