/// A `state` of the router.
pub trait State {}

/// A routing algorithm implementation.
pub trait Router<I: State> {
    type Ctx;
    type Err;

    /// Setup the routing algorithm.
    fn setup<'cx>(&mut self, ctx: &'cx Self::Ctx, state: I) -> Result<(), Self::Err>;

    /// Advance the routing algorithm.
    ///
    /// Returns the next `state` of the router.
    fn step<'cx>(&mut self, ctx: &'cx Self::Ctx, state: I) -> Result<I, Self::Err>;
}
