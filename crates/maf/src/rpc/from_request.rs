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

impl FromRequest for App {
    type Error = std::convert::Infallible;

    fn from_request(
        app: &App,
        _request: &mut RpcRequest,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> + Send {
        async { Ok(app.clone()) }
    }
}
