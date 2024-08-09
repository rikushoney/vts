use slotmap::{new_key_type, SlotMap};

use super::resources::{Bel, BelBucket, Group, Pip, Wire};

new_key_type! {
    pub struct BelId;
    pub struct WireId;
    pub struct PipId;
    pub struct GroupId;
    pub struct BelBucketId;
}

/// A `database` for device resources.
#[derive(Clone, Debug, Default)]
pub struct Database {
    bels: SlotMap<BelId, Bel>,
    wires: SlotMap<WireId, Wire>,
    pips: SlotMap<PipId, Pip>,
    groups: SlotMap<GroupId, Group>,
    buckets: SlotMap<BelBucketId, BelBucket>,
}

impl Database {
    /// Create a new empty database.
    pub fn new() -> Self {
        Self::default()
    }
}
