//! The primitive for observing changes in the application and triggering effects.
//!
//! When a dependency changes (e.g., a store is updated), we may need to trigger updates
//! to other parts of the application that depend on that data (e.g., recomputing selects).

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use maf_schemas::packet::{Bull, OneStoreUpdate, TxPacket};
use serde_json::Value;

use crate::{store::SelectKey, App};

#[derive(Debug, Default)]
pub struct ObserveStore {
    // pub(crate) select_dependencies: HashMap<TypeId, HashSet<SelectKey>>,
    targets: HashMap<ObserveDepdendency, HashSet<ObserveTarget>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObserveDepdendency {
    Store(TypeId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObserveTarget {
    Select(SelectKey),
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
    pub(crate) async fn trigger_update(
        &self,
        dependency: &ObserveDepdendency,
    ) -> anyhow::Result<()> {
        let dependents = match self.inner.observe.get_dependents(dependency) {
            Some(dependents) => dependents,
            None => return Ok(()),
        };

        let users = self.inner.state.users.read().await;

        for (_user_id, user) in users.iter() {
            let mut store_updates: Vec<OneStoreUpdate<Value>> = vec![];

            #[allow(irrefutable_let_patterns)] // TODO: remove when more dependency types are added
            if let ObserveDepdendency::Store(type_id) = dependency {
                // Notify the user that the store has been updated
                todo!();

                // let serializer = store.serializer.clone();
                // let data = store.data.read_owned().await;

                // let serialized_data = serializer.serialize(&data, &user).await?;

                // store_updates.push(OneStoreUpdate {
                //     store: &store.name,
                //     data: Bull::Owned(serialized_data),
                // });
            }

            for target in dependents {
                match target {
                    ObserveTarget::Select(select_key) => {
                        let content = self
                            .compute_select_contents(&select_key, user.clone())
                            .await?;

                        store_updates.push(OneStoreUpdate {
                            store: &select_key,
                            data: Bull::Owned(content),
                        });
                    }
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
