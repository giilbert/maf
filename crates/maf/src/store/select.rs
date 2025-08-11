use std::{any::TypeId, collections::HashSet, future::Future, pin::Pin, sync::Arc};

use crate::{callable::CallableFetch, App, Store, StoreData, User};

pub type SelectKey = Arc<str>;

#[allow(unused)]
pub struct AnySelect {
    pub(crate) name: SelectKey,
    pub(crate) select: Arc<
        dyn Fn(
                SelectContext,
            )
                -> Pin<Box<dyn Future<Output = Result<serde_json::Value, serde_json::Error>>>>
            + Send
            + Sync,
    >,
    /// NOTE: The *type id* of the store is used for dependency tracking instead of the *store key*
    /// because the store key is not available in the context of a select. This means that granular
    /// stores within the same type will not be tracked separately.
    ///
    /// TODO: Allow for selects on granular stores by somehow passing the store key in?
    pub(crate) depends_on_stores: HashSet<TypeId>,

    #[cfg(feature = "typed")]
    pub(crate) desc: crate::typed::StoreDesc,
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
    Store(TypeId),
    /// Not a dependency, used for types that do not cause selects to update.
    None,
}

pub trait SelectDependency {
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
        SelectDependencyType::Store(TypeId::of::<T>())
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
