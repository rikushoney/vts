/// A `database` for design entry resources.
#[derive(Clone, Debug, Default)]
pub struct Database {}

impl Database {
    /// Create a new empty database.
    pub fn new() -> Self {
        Self::default()
    }
}
