use std::sync::Arc;

#[cfg(feature = "typed")]
use schemars::SchemaGenerator;

use crate::{
    callable::{BoxedCallable, CallableFetch},
    store::StoreId,
    App, Store, StoreData, StoreMut, StoreRef, User,
};

pub type SelectKey = Arc<str>;

#[allow(unused)]
pub struct AnySelect {
    pub(crate) name: SelectKey,
    pub(crate) select: BoxedCallable<SelectContext, serde_json::Value, serde_json::Error>,
    #[cfg(feature = "typed")]
    pub(crate) desc:
        Arc<dyn Fn(&mut SchemaGenerator) -> crate::typed::StoreDesc + Send + Sync + 'static>,
}

pub struct SelectContext {
    pub(crate) app: App,
    pub(crate) user: User,
}

impl CallableFetch<App> for SelectContext {
    fn fetch(&self) -> App {
        self.app.clone()
    }
}

impl CallableFetch<User> for SelectContext {
    fn fetch(&self) -> User {
        self.user.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectDependencyType {
    /// The select depends on a store.
    Store(StoreId),
    /// Not a dependency, used for types that do not cause selects to update.
    None,
}

pub(crate) trait SelectDependency {
    #[allow(unused)]
    const IS_DEPENDENCY: bool = false;

    #[inline(always)]
    /// Returns the type id of the store that this select depends on.
    fn depends_on() -> SelectDependencyType {
        SelectDependencyType::None
    }
}

impl<T> SelectDependency for Store<T>
where
    T: StoreData,
{
    const IS_DEPENDENCY: bool = true;

    #[inline(always)]
    fn depends_on() -> SelectDependencyType {
        SelectDependencyType::Store(StoreId::of::<T>())
    }
}

impl<T> SelectDependency for StoreRef<T>
where
    T: StoreData,
{
    const IS_DEPENDENCY: bool = true;

    #[inline(always)]
    fn depends_on() -> SelectDependencyType {
        SelectDependencyType::Store(StoreId::of::<T>())
    }
}

impl<T> SelectDependency for StoreMut<T>
where
    T: StoreData,
{
    const IS_DEPENDENCY: bool = true;

    #[inline(always)]
    fn depends_on() -> SelectDependencyType {
        SelectDependencyType::Store(StoreId::of::<T>())
    }
}

macro_rules! impl_not_dependency {
    ($($t:ty),+) => {
        $(impl SelectDependency for $t {})+
    };
}

impl_not_dependency!(App, User);

pub trait GetParamSelectDependencies<const N: usize> {
    fn get_select_dependencies() -> [SelectDependencyType; N];
}

macro_rules! extract_select_dependency {
    ($n:expr, $($members:ident),+) => {
        impl<
            $($members),+
        > GetParamSelectDependencies<$n>
            for ($($members,)+)
        where
            $($members: SelectDependency),+
        {
            #[inline(always)]
            fn get_select_dependencies() -> [SelectDependencyType; $n] {
                [
                    $($members::depends_on()),+,
                ]
            }
        }
    };
}

extract_select_dependency!(1, T1);
extract_select_dependency!(2, T1, T2);
extract_select_dependency!(3, T1, T2, T3);
extract_select_dependency!(4, T1, T2, T3, T4);
extract_select_dependency!(5, T1, T2, T3, T4, T5);
extract_select_dependency!(6, T1, T2, T3, T4, T5, T6);
extract_select_dependency!(7, T1, T2, T3, T4, T5, T6, T7);
extract_select_dependency!(8, T1, T2, T3, T4, T5, T6, T7, T8);
