use crate::App;

use super::RpcRequest;

pub trait FromRequest
where
    Self: Sized,
{
    type Error: std::error::Error + Send + Sync + 'static;

    fn from_request(
        app: &App,
        request: &mut RpcRequest,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> + Send;
}
