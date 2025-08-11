use facet::Facet;
use serde::de::DeserializeOwned;

use crate::{callable::IntoCallable, store::SelectContext, App, Params, Store, StoreData, User};

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

pub trait ExtractRpcDesc<Params, Ret, const IS_ASYNC: bool, const IS_RESULT: bool = false> {
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

trait RpcResult<const IS_RESULT: bool> {
    const SHAPE: &'static facet::Shape;
}

impl<T> RpcResult<false> for T
where
    T: for<'a> Facet<'a>,
{
    const SHAPE: &'static facet::Shape = T::SHAPE;
}

impl<T, E> RpcResult<true> for Result<T, E>
where
    T: for<'a> Facet<'a>,
{
    const SHAPE: &'static facet::Shape = T::SHAPE;
}

// Case where the type is a function that takes no parameters
impl<Ret, F, const IS_RESULT: bool> ExtractRpcDesc<(), Ret, false, IS_RESULT> for F
where
    Ret: RpcResult<IS_RESULT>,
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

impl<Ret, Fut, F, const IS_RESULT: bool> ExtractRpcDesc<(), Ret, true, IS_RESULT> for F
where
    Ret: RpcResult<IS_RESULT>,
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
        impl<
            $($members),+,
            Ret,
            F,
            const IS_RESULT: bool
        > ExtractRpcDesc<($($members,)+), Ret, false, IS_RESULT> for F
        where
            $($members: GetParamFacet),+,
            Ret: RpcResult<IS_RESULT>,
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

        impl<
            $($members),+,
            Ret,
            Fut,
            F,
            const IS_RESULT: bool
        > ExtractRpcDesc<($($members,)+), Ret, true, IS_RESULT> for F
        where
            $($members: GetParamFacet),+,
            Ret: RpcResult<IS_RESULT>,
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

pub trait ExtractSelectDesc<Params, Ret, const IS_ASYNC: bool> {
    fn extract(name: String) -> StoreDesc;
}

impl<F, Params, Ret, const IS_ASYNC: bool> ExtractSelectDesc<Params, Ret, IS_ASYNC> for F
where
    F: IntoCallable<SelectContext, Params, Ret, std::convert::Infallible, (), IS_ASYNC>,
    Ret: facet::Facet<'static>,
{
    fn extract(name: String) -> StoreDesc {
        let result = Ret::SHAPE;
        StoreDesc {
            name,
            select: result,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        typed::{desc::ExtractRpcDesc, ExtractSelectDesc},
        Params, Store, StoreData,
    };

    struct Test;
    impl StoreData for Test {
        type Select<'this> = i32;

        fn init() -> Self {
            Test
        }

        fn select(&self, _user: &crate::User) -> Self::Select<'_> {
            1
        }
    }

    fn is_extract_rpc_desc<F, Params, Ret, const IS_ASYNC: bool>(_fn: F)
    where
        F: ExtractRpcDesc<Params, Ret, IS_ASYNC>,
    {
    }

    #[test]
    fn extract_rpc_desc_compiles() {
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

    fn is_extract_select_desc<F, Params, Ret, const IS_ASYNC: bool>(_fn: F)
    where
        F: ExtractSelectDesc<Params, Ret, IS_ASYNC>,
    {
    }

    #[test]
    fn extract_select_desc_compiles() {
        fn select_test_store(_store: Store<Test>) -> i32 {
            42
        }
        async fn async_select_test_store(_store: Store<Test>) -> i32 {
            42
        }

        is_extract_select_desc(select_test_store);
        is_extract_select_desc(async_select_test_store);
    }
}
