use axum::extract::{FromRequestParts, State, WebSocketUpgrade};

use crate::axum::Room;

pub struct MafRoutes<T: RouteConfig> {
    _marker: std::marker::PhantomData<T>,
}

pub trait RouteConfig: Sized + Clone + Send + Sync + 'static {
    type Params: FromRequestParts<Self> + Send;

    fn get_room(&self, params: Self::Params) -> Option<Room>;
}

impl<T: RouteConfig> MafRoutes<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn connect_handler(&self) -> axum::Router<T> {
        axum::Router::new().route(
            "/connect",
            axum::routing::get(
                |params: T::Params, ws: WebSocketUpgrade, state: State<T>| async move {
                    let room = state.get_room(params);

                    match room {
                        Some(room) => Ok(ws.on_upgrade(|socket| async move {
                            if let Err(e) = room.handle_upgrade(socket).await {
                                tracing::error!("error during WebSocket connection: {e:?}");
                            }
                        })),
                        None => Err(axum::http::StatusCode::NOT_FOUND),
                    }
                },
            ),
        )
    }
}
