use crate::{rpc::RpcRequestContext, App};

use super::CallableParam;

pub trait CtxWithApp {
    fn app(&self) -> &App;
}

impl CtxWithApp for RpcRequestContext {
    fn app(&self) -> &App {
        &self.app
    }
}

impl<Ctx: CtxWithApp + Send, Init: Sync> CallableParam<Ctx, Init> for App {
    type Error = std::convert::Infallible;

    async fn extract(ctx: &mut Ctx, _: &Init) -> Result<Self, Self::Error> {
        Ok(ctx.app().clone())
    }
}
