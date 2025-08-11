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

        bindings::bindgen::report_app_schema(
            &serde_json::to_string_pretty(&maf_schemas::typed::AppSchema {
                rpcs: rpcs
                    .values()
                    .map(|rpc| maf_schemas::typed::RpcSerialized {
                        name: rpc.method.to_string(),
                        params: rpc.desc.params.map(|p| p.into()),
                        result: Some(rpc.desc.result.into()),
                    })
                    .collect(),
                stores: stores
                    .iter()
                    .map(|(_, store)| maf_schemas::typed::StoreSerialized {
                        name: store.desc.name.clone(),
                        select: store.desc.select.into(),
                    })
                    // Selects behave like stores client-side, so we can include them here
                    .chain(self.inner.selects.iter().map(|(_, select)| {
                        maf_schemas::typed::StoreSerialized {
                            name: select.desc.name.clone(),
                            select: select.desc.select.into(),
                        }
                    }))
                    .collect(),
            })
            .expect("Failed to serialize schema"),
        );
    }
}
