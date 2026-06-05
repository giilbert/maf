//! Utilities for registering [`crate::AppBuilder::on_connect`] and
//! [`crate::AppBuilder::on_disconnect`] handlers.

use super::local::LocalStateError;
use crate::callable::{BoxedCallable, CallableFetch};
use crate::{App, User};

pub type OnConnectDisconnectFn =
    BoxedCallable<OnConnectDiconnectContext, (), OnConnectDisconnectError>;

pub struct OnConnectDiconnectContext {
    pub app: App,
    pub user: User,
}

#[derive(Debug, thiserror::Error)]
pub enum OnConnectDisconnectError {
    #[error("state error: {0}")]
    State(#[from] LocalStateError),
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
    use super::*;
    use crate::callable::CallableParam;
    use crate::{Store, StoreData};

    #[test]
    fn type_checks() {
        const fn check_on_connect_disconnect_parameter<
            T: CallableParam<OnConnectDiconnectContext, ()>,
        >() {
        }

        struct Counter {
            count: i32,
        }

        impl StoreData for Counter {
            type Select<'this> = i32;

            fn init() -> Self {
                Counter { count: 0 }
            }

            fn select(&self, _user: &User) -> Self::Select<'_> {
                self.count
            }
        }

        check_on_connect_disconnect_parameter::<Store<Counter>>();
        check_on_connect_disconnect_parameter::<User>();
        check_on_connect_disconnect_parameter::<App>();
    }
}
