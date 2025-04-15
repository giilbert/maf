use std::{future::Future, pin::Pin};

pub type UnitFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
