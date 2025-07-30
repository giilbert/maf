use facet::Facet;

use crate::StoreData;

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
    pub params: &'static facet::Shape,
    pub result: &'static facet::Shape,
}
