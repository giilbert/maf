use schemas::typed::{AppSchema, StoreSerialized, TypeKind};

#[derive(Debug)]
pub struct TypeScriptCodegen {
    pub(crate) schema: AppSchema,
}

impl TypeScriptCodegen {
    pub fn emit(&self) -> String {
        let mut output = String::new();

        output
    }
}

#[cfg(test)]
mod tests {
    use facet::Facet;
    use maf::StoreData;
    use serde::Serialize;

    use super::*;

    fn store_to_schema<T: StoreData>() -> StoreSerialized {
        StoreSerialized {
            name: T::name().as_ref().to_string(),
            select: TypeKind::from(T::Select::SHAPE),
        }
    }

    #[test]
    fn typescript_pomodoro() {
        #[derive(Serialize, Facet)]
        pub struct Pomodoro {
            pub phase: String,
            pub count: u32,
            pub auto: bool,
        }

        impl StoreData for Pomodoro {
            type Select<'this> = &'this Pomodoro;

            fn init() -> Self {
                Self {
                    phase: "hustle".to_string(),
                    count: 0,
                    auto: false,
                }
            }

            fn select(&self, _user: &maf::User) -> Self::Select<'_> {
                self
            }
        }

        let codegen = TypeScriptCodegen {
            schema: AppSchema {
                rpcs: vec![],
                stores: vec![store_to_schema::<Pomodoro>()],
            },
        };

        println!("{codegen:#?}");

        panic!();
    }
}
