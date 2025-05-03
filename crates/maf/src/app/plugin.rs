use super::App;

pub trait Plugin {
    fn build(&self, app: App) -> App;
}

impl App {
    pub fn plugin(self, plugin: impl Plugin) -> Self {
        plugin.build(self)
    }
}
