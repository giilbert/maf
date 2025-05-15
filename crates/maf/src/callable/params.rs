pub trait CallableParam<Ctx, Init>: Sized {
    type Error: std::error::Error + Send;

    fn extract(
        ctx: &mut Ctx,
        init: &Init,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> + Send;
}
