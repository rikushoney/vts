use ustr::Ustr;

use super::database::{BelId, GroupId, PipId, WireId};

/// A basic element.
#[derive(Clone, Debug)]
pub struct Bel {
    name: Ustr,
}

/// A physical connection between [Pip]'s and/or [Bel] pins.
#[derive(Clone, Debug)]
pub struct Wire {
    name: Ustr,
}

/// A programmable interconnect point.
#[derive(Clone, Debug)]
pub struct Pip {
    name: Ustr,
}

/// An item within a [Group].
#[derive(Clone, Debug)]
pub enum GroupItem {
    Bel(BelId),
    Pip(PipId),
    Wire(WireId),
    Group(GroupId),
}

/// A collection of [Bel]'s, [Pip]'s, [Wire]'s and/or other [Group]'s.
#[derive(Clone, Debug)]
pub struct Group {
    name: Ustr,
    items: Vec<GroupItem>,
}

/// A collection of [Bel]'s and `cell types`.
#[derive(Clone, Debug)]
pub struct BelBucket;
