mod desc;

pub use desc::{ExtractRpcDesc, ExtractSelectDesc, RpcDesc, StoreDesc};

use crate::{bindings, App};

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

        bindings::bindgen::report_app_schema(
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
