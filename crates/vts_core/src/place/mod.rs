/// A `state` of the placer.
pub trait State {}

/// A placement algorithm implementation.
pub trait Placer<I: State> {
    type Ctx;
    type Err;

    /// Setup the placement algorithm.
    fn setup<'cx>(&mut self, ctx: &'cx Self::Ctx, state: I) -> Result<(), Self::Err>;

    /// Advance the placement algorithm.
    ///
    /// Returns the next `state` of the placer.
    fn step<'cx>(&mut self, ctx: &'cx Self::Ctx, state: I) -> Result<I, Self::Err>;
}
