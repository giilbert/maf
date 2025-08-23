pub struct MafRoutes<T: RouteConfig> {
    _marker: std::marker::PhantomData<T>,
}

pub trait RouteConfig {}

impl<T: RouteConfig> MafRoutes<T> {}
