use axum::extract::{FromRequestParts, State, WebSocketUpgrade};

use crate::axum::Room;

/// Used to create routes for a Cobble server.
///
/// The generic parameter `S` should your application's state type, which also must implement
/// [`RouteConfig`]. This state type is used to provide customizability to Cobble, e.g. a way to look
/// up a [`Room`] instance based on request parameters.
///
/// In order for a Cobble client to connect, you must provide a `/connect` route, which can be done by
/// nesting a router created by [`CobbleRoutes::connect_handler`] into some path in your own router.
pub struct CobbleRoutes<S: RouteConfig> {
    _marker: std::marker::PhantomData<S>,
}

/// A trait that must be implemented by your application's state type in order to use [`CobbleRoutes`].
pub trait RouteConfig: Sized + Clone + Send + Sync + 'static {
    type GetRoomParams: FromRequestParts<Self> + Send;
    /// Given the parameters extracted from a `GET /connect` request, return the corresponding
    /// [`Room`] instance, or `None` if no such room exists and/or an error occurred.
    fn get_room(
        &self,
        params: Self::GetRoomParams,
    ) -> impl Future<Output = Option<Room>> + Send + Sync;
}

impl<S: RouteConfig> CobbleRoutes<S> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn connect_handler(&self) -> axum::Router<S> {
        axum::Router::new().route(
            "/connect",
            axum::routing::get(
                |params: S::GetRoomParams, ws: WebSocketUpgrade, state: State<S>| async move {
                    let room = state.get_room(params).await;

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
