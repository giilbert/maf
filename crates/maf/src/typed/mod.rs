//! Utilities for serializing and exporting type information about MAF apps.
//!
//! The features described by this trait are minimally intrusive and optional (enabled through the
//! `typed` feature). To use this feature, the types being exported (RPC parameters, RPC return,
//! and store value types) must implement [`schemars::JsonSchema`], a trait that provides type
//! introspection using JSON Schema. Most primitive types already implement this trait, and for
//! custom types, it can be implemented using the derive macro.
//!
//! [`schemars::JsonSchema`] is compatible with [`serde`], recognizing `#[serde]` attributes like
//! field renames, enum tagging, defaults, and so on. See the [`schemars` documentation]
//! (https://graham.cool/schemars/examples/2-serde_attrs) for more information.
//!
//! ## Example
//!
//! ```rust
//! #[derive(serde::Serialize, schemars::JsonSchema)]
//! #[serde(rename_all = "camelCase")]
//! struct Game {
//!     points: i32,
//!     round: u32,
//!     phase: String,
//!     // The `next_phase` field will be serialized as `nextPhase`
//!     next_phase: Option<String>,
//! }
//!
//! impl StoreData for Game {
//!     type Select<'this> = &'this Game;
//!     //                   ^^^^^^^^^^^ Store::Select must implement `JsonSchema`
//!     // ...
//! }
//!
//! fn make_move(params: Params<(u32, u32)>) -> String {
//!     //                      ^^^^^^^^^^      ^^^^^^
//!     //          Parameter and return types must implement `JsonSchema`
//!     // ...
//! }
//!
//! fn build() -> App {
//!     App::builder()
//!         // Types should be statically registered in the app builder
//!         .store::<Game>()
//!         .rpc("make_move", make_move)
//!         // ...
//!         .build()
//! }
//!
//! ```
//!
//! ## ⚠️ Work in Progress ⚠️
//!
//! Currently, only types for RPCs and stores are exported. This is used for generating types for
//! the client (e.g. in TypeScript, this allows for type-safe RPC calls and store access).

mod desc;

pub(crate) use desc::{ExtractRpcDesc, ExtractSelectDesc};
pub use desc::{RpcDesc, StoreDesc};

use crate::{platform::Platform, App};

impl App {
    pub(crate) fn export_types(&self) {
        let stores = self
            .inner
            .state
            .stores
            .try_read()
            .expect("Failed to read stores");
        let rpcs = &self.inner.rpc_functions.inner;

        // for (key, store) in stores.iter() {
        //     println!("store: key = {:?}, desc = {:?}", key.as_ref(), store.desc);
        // }

        // for rpc in rpcs.values() {
        //     println!("rpc: method = {:?} desc = {:?}", rpc.method, rpc.desc);
        // }

        let mut generator = schemars::SchemaGenerator::default();
        let select_stores = self
            .inner
            .selects
            .iter()
            .map(|(_, select)| {
                let desc = (select.desc)(&mut generator);
                maf_schemas::typed::StoreSerialized {
                    name: desc.name,
                    select: desc.select,
                }
            })
            .collect::<Vec<_>>();

        let stores = stores
            .iter()
            .map(|(_, store)| {
                let desc = (store.desc)(&mut generator);
                maf_schemas::typed::StoreSerialized {
                    name: desc.name,
                    select: desc.select,
                }
            })
            // Selects behave like stores client-side, so we can include them here
            .chain(select_stores.into_iter())
            .collect();

        self.inner.platform.report_app_schema(
            &serde_json::to_string_pretty(&maf_schemas::typed::AppSchema {
                rpcs: rpcs
                    .values()
                    .map(|rpc| {
                        let desc = (rpc.desc)(&mut generator);
                        maf_schemas::typed::RpcSerialized {
                            name: rpc.method.to_string(),
                            params: desc.params,
                            result: desc.result,
                        }
                    })
                    .collect(),
                stores,
            })
            .expect("Failed to serialize schema"),
        );
    }
}
