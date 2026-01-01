use axum::extract::ws::WebSocket;
use maf::{
    app::AppBuilder,
    platform::{ActorPlatformHandle, Platform, TargetPlatform as ActorPlatform},
    prelude::Uuid,
};

use crate::axum::Connection;

#[derive(Clone)]
pub struct Room {
    _id: Uuid,
    platform: ActorPlatformHandle,
}

impl Room {
    pub fn new(builder: AppBuilder) -> anyhow::Result<Self> {
        let platform = ActorPlatform::init(())?;

        let handle = platform.handle();
        let builder = builder.platform(platform);

        let app = builder.build();
        // TODO: this just makes it run in a background task, should handle errors maybe?
        app.run();

        Ok(Self {
            _id: Uuid::new_v4(),
            platform: handle,
        })
    }

    pub async fn handle_upgrade(
        &self,
        ws: WebSocket,
        auth_data: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        let connection = Connection::new(ws, auth_data).await?;
        self.platform.add_user(connection.platform)?;
        // TODO: allow connection to be used in the future instead of being dropped here
        Ok(())
    }
}
