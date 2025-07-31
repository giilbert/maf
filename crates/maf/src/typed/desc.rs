use facet::Facet;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{App, Params, Store, StoreData, User};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreDesc {
    pub name: String,
    pub select: &'static facet::Shape,
}

impl StoreDesc {
    pub fn new<T>() -> Self
    where
        T: StoreData,
    {
        StoreDesc {
            name: T::name().as_ref().to_string(),
            select: T::Select::SHAPE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RpcDesc {
    pub name: String,
    pub params: Option<&'static facet::Shape>,
    pub result: &'static facet::Shape,
}

pub trait ExtractRpcDesc<Params, Ret, const IS_ASYNC: bool> {
    fn extract(name: String) -> RpcDesc;
}

trait GetParamFacet {
    const IS_PARAM: bool = false;
    fn get_param_facet() -> &'static facet::Shape {
        panic!("get_param_facet called on non-param type")
    }
}

impl<T> GetParamFacet for Params<T>
where
    T: DeserializeOwned + for<'a> Facet<'a>,
{
    const IS_PARAM: bool = true;

    fn get_param_facet() -> &'static facet::Shape {
        T::SHAPE
    }
}

// Since specialization does not exist in stable Rust, a macro is used to implement the trait for
// types that is not `Params<T>`.
macro_rules! impl_not_param {
    // Case where the type does not have a type parameter
    ($t:ty) => {
        impl GetParamFacet for $t {
            const IS_PARAM: bool = false;
        }
    };

    // Case where the type has a type parameter
    ($t:ty, $($param:tt)*) => {
        impl<$($param)*> GetParamFacet for $t {
            const IS_PARAM: bool = false;
        }
    };
}

impl_not_param!(App);
impl_not_param!(User);
impl_not_param!(Store<T>, T: StoreData);

// Case where the type is a function that takes no parameters
impl<Ret, F> ExtractRpcDesc<(), Ret, false> for F
where
    Ret: for<'a> Facet<'a>,
    F: Fn() -> Ret,
{
    fn extract(name: String) -> RpcDesc {
        RpcDesc {
            name,
            params: None,
            result: Ret::SHAPE,
        }
    }
}

impl<Ret, Fut, F> ExtractRpcDesc<(), Ret, true> for F
where
    Ret: for<'a> Facet<'a>,
    Fut: std::future::Future<Output = Ret>,
    F: Fn() -> Fut,
{
    fn extract(name: String) -> RpcDesc {
        RpcDesc {
            name,
            params: None,
            result: Ret::SHAPE,
        }
    }
}

macro_rules! impl_extract_rpc_desc {
    ($($members:ident),+) => {
        impl<$($members),+, Ret, F> ExtractRpcDesc<($($members,)+), Ret, false> for F
        where
            $($members: GetParamFacet),+,
            Ret: for<'a> Facet<'a>,
            F: Fn($($members),+) -> Ret,
        {
            fn extract(name: String) -> RpcDesc {
                let params = if false { None } $(else if $members::IS_PARAM {
                    Some($members::get_param_facet())
                })* else {
                    None
                };

                RpcDesc {
                    name,
                    params,
                    result: Ret::SHAPE,
                }
            }
        }

        impl<$($members),+, Ret, Fut, F> ExtractRpcDesc<($($members,)+), Ret, true> for F
        where
            $($members: GetParamFacet),+,
            Ret: for<'a> Facet<'a>,
            Fut: std::future::Future<Output = Ret>,
            F: Fn($($members),+) -> Fut,
        {
            fn extract(name: String) -> RpcDesc {
                let params = if false { None } $(else if $members::IS_PARAM {
                    Some($members::get_param_facet())
                })* else {
                    None
                };

                RpcDesc {
                    name,
                    params,
                    result: Ret::SHAPE,
                }
            }
        }
    };
}

impl_extract_rpc_desc!(T1);
impl_extract_rpc_desc!(T1, T2);
impl_extract_rpc_desc!(T1, T2, T3);
impl_extract_rpc_desc!(T1, T2, T3, T4);
impl_extract_rpc_desc!(T1, T2, T3, T4, T5);
impl_extract_rpc_desc!(T1, T2, T3, T4, T5, T6);
impl_extract_rpc_desc!(T1, T2, T3, T4, T5, T6, T7);
impl_extract_rpc_desc!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_extract_rpc_desc!(T1, T2, T3, T4, T5, T6, T7, T8, T9);

#[cfg(test)]
mod tests {
    use crate::{typed::desc::ExtractRpcDesc, Params, Store, StoreData};

    fn is_extract_rpc_desc<F, Params, Ret, const IS_ASYNC: bool>(_fn: F)
    where
        F: ExtractRpcDesc<Params, Ret, IS_ASYNC>,
    {
    }

    #[test]
    fn extract_rpc_desc_compiles() {
        struct Test;
        impl StoreData for Test {
            type Select = i32;

            fn init() -> Self {
                Test
            }

            fn select(&self, _user: &crate::User) -> &Self::Select {
                &1
            }
        }

        fn string_to_string_fn(_params: Params<String>) -> String {
            "hello".to_string()
        }
        fn string_to_unit_fn(_params: Params<String>) {}
        async fn with_store_fn(_params: Params<String>, _store: Store<Test>) -> i32 {
            0
        }
        fn unit_to_unit_fn() {}

        async fn async_string_to_string_fn(_params: Params<String>) -> String {
            "hello".to_string()
        }

        is_extract_rpc_desc(string_to_string_fn);
        is_extract_rpc_desc(string_to_unit_fn);
        is_extract_rpc_desc(with_store_fn);
        is_extract_rpc_desc(unit_to_unit_fn);
        is_extract_rpc_desc(async_string_to_string_fn);
    }
}
