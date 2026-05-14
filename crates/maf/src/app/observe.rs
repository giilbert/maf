//! The primitive for observing changes in the application and triggering effects.
//!
//! When a dependency changes (e.g., a store is updated), we may need to trigger updates
//! to other parts of the application that depend on that data (e.g., recomputing selects).

use std::collections::{HashMap, HashSet};

use maf_schemas::packet::{Bull, OneStoreUpdate, TxPacket};
use serde_json::Value;

use crate::{
    app::meta::MetaKey,
    store::{SelectKey, StoreId},
    App,
};

#[derive(Debug, Default)]
pub struct ObserveStore {
    // pub(crate) select_dependencies: HashMap<TypeId, HashSet<SelectKey>>,
    targets: HashMap<ObserveDepdendency, HashSet<ObserveTarget>>,
}

#[non_exhaustive] // TODO: remove when adding more dependency types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ObserveDepdendency {
    Store(StoreId),
    Users,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ObserveTarget {
    Select(SelectKey),
    Meta(MetaKey),
}

impl ObserveStore {
    pub(crate) fn add_dependency(&mut self, dependency: ObserveDepdendency, target: ObserveTarget) {
        self.targets.entry(dependency).or_default().insert(target);
    }

    pub(crate) fn get_dependents(
        &self,
        dependency: &ObserveDepdendency,
    ) -> Option<&HashSet<ObserveTarget>> {
        self.targets.get(dependency)
    }
}

impl App {
    /// Triggers an update for the referenced `dependency` and all dependents of the given
    /// dependency.
    ///
    /// TODO: This assumes that the tree of dependencies is only one level deep (i.e., a store
    /// update only triggers select updates, and select updates do not trigger other select updates
    /// or store updates). This is sufficient for now as stores cannot trigger other stores, but in
    /// the future we may want to support more complex dependency graphs.
    pub(crate) async fn trigger_update(
        &self,
        dependency: &ObserveDepdendency,
    ) -> anyhow::Result<()> {
        let dependents = self.inner.observe.get_dependents(dependency);

        let users = self.inner.state.users.read().await;
        let store_dependency = match dependency {
            ObserveDepdendency::Store(store_id) => Some(self.get_any_store(store_id).await?),
            #[allow(unreachable_patterns)]
            _ => None,
        };

        // These types of dependencies do not affect users and only run once on the app level
        for target in dependents.into_iter().flatten() {
            if let ObserveTarget::Meta(meta_key) = target {
                self.inner
                    .meta
                    .trigger_meta_update(self.clone(), meta_key)
                    .await?;
            }
        }

        for (_user_id, user) in users.iter() {
            let mut store_updates: Vec<OneStoreUpdate<Value>> = vec![];

            // Indicate that the store has changed (the start of the update)
            if let Some(store) = &store_dependency {
                store_updates.push(OneStoreUpdate {
                    store: &store.name,
                    data: Bull::Owned(self.serialize_store(user.clone(), store.clone()).await?),
                });
            }

            for target in dependents.into_iter().flatten() {
                if let ObserveTarget::Select(select_key) = target {
                    let content = self
                        .compute_select_contents(select_key, user.clone())
                        .await?;

                    store_updates.push(OneStoreUpdate {
                        store: &select_key.0,
                        data: Bull::Owned(content),
                    });
                }
            }

            if !store_updates.is_empty() {
                user.send(TxPacket::ManyStoreUpdate::<()>(store_updates))
                    .ok();
            }
        }

        Ok(())
    }
}
