use super::app::AppBuilder;

pub trait Plugin {
    fn build(&self, app: AppBuilder) -> AppBuilder;
}

impl AppBuilder {
    pub fn plugin(self, plugin: impl Plugin) -> Self {
        plugin.build(self)
    }
}
