use super::app::AppBuilder;

/// Modifies an [`AppBuilder`] to add functionality to a [`crate::App`].
///
/// This is useful for packaging related functionality together, such as a set of RPC functions,
/// stores, and background tasks that implement a specific feature or system. Plugins can be used to
/// compose complex applications from smaller, reusable components and export/publish them.
pub trait Plugin {
    /// Adds functionality via the provided [`AppBuilder`].
    fn build(&self, app: AppBuilder) -> AppBuilder;
}

impl AppBuilder {
    /// Registers a plugin to modify the application.
    ///
    /// # Examples
    /// ```rust
    /// use maf::prelude::*;
    ///
    /// // A plugin that adds game functionality to the app.
    /// // NOTE: options and configuration can be passed to the plugin via its fields.
    /// pub struct GamePlugin {
    ///     total_rounds: u32,
    /// }
    ///
    /// // ... other plugin code ...
    ///
    /// impl Plugin for GamePlugin {
    ///     fn build(&self, app: AppBuilder) -> AppBuilder {
    ///         // Initialize game state, RPCs, stores, etc.
    ///         app
    ///             // .store::<GameState>()
    ///             // .rpc("start_round", start_round)
    ///             // ...
    ///     }
    /// }
    ///
    /// // ... where the app is built ...
    /// fn build() -> App {
    ///     App::builder()
    ///         // Consume the plugin to modify the app
    ///         .plugin(GamePlugin { total_rounds: 5 })
    ///         // ... other app configuration ...
    ///         .build()
    /// }
    ///
    /// maf::register!(build);
    /// ```
    pub fn plugin(self, plugin: impl Plugin) -> Self {
        plugin.build(self)
    }
}
