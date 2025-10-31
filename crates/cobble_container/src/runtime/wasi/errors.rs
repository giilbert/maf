/// A newtype wrapper around the WIT-defined ListenError with support for trapping and conversions.
pub struct ListenError(pub wasmtime_wasi::TrappableError<super::bindings::ListenError>);

impl From<wasmtime::component::ResourceTableError> for ListenError {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self(wasmtime_wasi::TrappableError::trap(error))
    }
}

impl From<wasmtime::Trap> for ListenError {
    fn from(error: wasmtime::Trap) -> Self {
        Self(wasmtime_wasi::TrappableError::trap(error))
    }
}

impl From<anyhow::Error> for ListenError {
    fn from(value: anyhow::Error) -> Self {
        Self(wasmtime_wasi::TrappableError::trap(value))
    }
}

impl From<super::bindings::ListenError> for ListenError {
    fn from(value: super::bindings::ListenError) -> Self {
        Self(wasmtime_wasi::TrappableError::trap(value))
    }
}
