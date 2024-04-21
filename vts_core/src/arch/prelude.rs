pub use super::{
    component::{Component, ComponentAccess, ComponentClass},
    connection::{Connection, ConnectionKind},
    module::Module,
    port::{Port, PortAccess, PortClass, PortKind},
    reference::{ComponentRef, ComponentRefAccess},
};

pub(crate) use super::{
    checker::Checker,
    component::ComponentData,
    linker::Linker,
    module::{ComponentId, ComponentRefId, ModuleLookup, PortId},
    port::PortData,
    reference::ComponentRefData,
};
