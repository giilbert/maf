use axum::extract::ws::WebSocket;
use maf::{
    AppBuilder, Uuid,
    platform::{ActorPlatformHandle, Platform, TargetPlatform as ActorPlatform},
};

use crate::axum::Connection;

pub struct Room {
    id: Uuid,
    platform: ActorPlatformHandle,
}

impl Room {
    pub fn new(builder: AppBuilder) -> anyhow::Result<Self> {
        let platform = ActorPlatform::init(())?;

        let handle = platform.handle();
        let builder = builder.platform(platform);

        let app = builder.build();

        Ok(Self {
            id: Uuid::new_v4(),
            platform: handle,
        })
    }

    pub async fn handle_upgrade(&self, ws: WebSocket) -> anyhow::Result<()> {
        let connection = Connection::new(ws).await?;

        Ok(())
    }
}
