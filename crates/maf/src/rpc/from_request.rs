use crate::App;

use super::RpcRequest;

pub trait FromRequest
where
    Self: Sized,
{
    fn from_request(
        app: &App,
        request: &mut RpcRequest,
    ) -> impl std::future::Future<Output = anyhow::Result<Self>> + Send;
}
