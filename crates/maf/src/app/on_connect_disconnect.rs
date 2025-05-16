use crate::{
    callable::{AnyCallable, CallableFetch},
    App, User,
};

pub type OnConnectDisconnectFn =
    AnyCallable<OnConnectDiconnectContext, (), OnConnectDisconnectError>;

pub struct OnConnectDiconnectContext {
    pub app: App,
    pub user: User,
}

#[derive(Debug, thiserror::Error)]
pub enum OnConnectDisconnectError {
    #[error("infalliable error: {0}")]
    Infalliable(#[from] std::convert::Infallible),
}

impl CallableFetch<App> for OnConnectDiconnectContext {
    fn fetch(&self) -> App {
        self.app.clone()
    }
}

impl CallableFetch<User> for OnConnectDiconnectContext {
    fn fetch(&self) -> User {
        self.user.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::{callable::CallableParam, Store, StoreData};

    use super::*;

    #[test]
    fn type_checks() {
        const fn check_on_connect_disconnect_parameter<
            T: CallableParam<OnConnectDiconnectContext, ()>,
        >() {
        }

        struct T {}

        impl StoreData for T {
            type Data = i32;

            fn init() -> Self::Data {
                42
            }
        }

        check_on_connect_disconnect_parameter::<Store<T>>();
        check_on_connect_disconnect_parameter::<User>();
        check_on_connect_disconnect_parameter::<App>();
    }
}
