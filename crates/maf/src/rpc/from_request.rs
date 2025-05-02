use crate::app::AppState;

use super::RpcRequest;

pub trait FromRequest
where
    Self: Sized,
{
    fn from_request(
        state: &AppState,
        request: &mut RpcRequest,
    ) -> impl std::future::Future<Output = anyhow::Result<Self>> + Send;
}
