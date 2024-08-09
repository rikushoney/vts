pub trait Router {
    type Ctx;
    type Err;

    fn run<'cx>(ctx: &'cx Self::Ctx) -> Result<(), Self::Err>;
}
