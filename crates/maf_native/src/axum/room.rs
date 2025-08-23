use maf::{
    AppBuilder,
    platform::{ActorPlatformHandle, Platform, TargetPlatform as ActorPlatform},
};

pub struct Room {
    platform: ActorPlatformHandle,
}

impl Room {
    pub fn new(app: AppBuilder) -> anyhow::Result<Self> {
        let platform = ActorPlatform::init(())?;
        let handle = platform.handle();
        app.platform(platform);

        Ok(Self { platform: handle })
    }
}
