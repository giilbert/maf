use std::any::TypeId;

pub(crate) struct RpcFunction {
    pub(crate) path: String,
    pub(crate) type_id: TypeId,
    pub(crate) handler: Box<dyn Fn() -> ()>,
}

impl std::fmt::Debug for RpcFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcFunction")
            .field("path", &self.path)
            .field("type_id", &self.type_id)
            .finish()
    }
}

#[repr(C)]
pub(crate) struct RpcRequest<'a> {
    path: String,
    data: &'a [u8],
}
