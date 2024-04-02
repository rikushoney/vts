use crate::stringtable::TableKey;

#[macro_use]
mod macros;

pub mod component;
pub mod module;
pub mod port;

pub use component::{ComponentClass, ComponentId};
pub use module::Module;
pub use port::{PortClass, PortId, PortKind};

impl_opaquekey_wrapper!(StringId, u32);

impl TableKey for StringId {}
