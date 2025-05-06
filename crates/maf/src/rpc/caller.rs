use crate::User;

use super::FromRequest;

impl FromRequest for User {
    type Error = std::convert::Infallible;

    async fn from_request(
        _app: &crate::App,
        request: &mut crate::rpc::RpcRequest,
    ) -> Result<Self, Self::Error> {
        Ok(request.user.clone())
    }
}
