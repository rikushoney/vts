use vts_shared::{stringtable::TableKey, OpaqueKey};

pub use crate::component::{Component, ComponentClass};
pub use crate::module::Module;
use crate::module::{ComponentId, PortId};
pub use crate::port::{Port, PortClass, PortKind};

// TODO: eventually make this a derive macro in vts_shared
macro_rules! impl_opaquekey_wrapper {
    ($name:ident, $base:path) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub(crate) struct $name($base);

        impl $crate::OpaqueKey for $name {
            fn as_index(&self) -> usize {
                self.0.as_index()
            }

            fn from_index(idx: usize) -> Self {
                $name(<$base as $crate::OpaqueKey>::from_index(idx))
            }

            fn max_index() -> usize {
                <$base as $crate::OpaqueKey>::max_index()
            }
        }
    };
}

macro_rules! impl_dbkey_wrapper {
    ($name:ident, $base:path) => {
        impl_opaquekey_wrapper!($name, $base);

        impl vts_shared::database::DbKey for $name {}
    };
}

pub(crate) use impl_dbkey_wrapper;

mod component;
mod module;
mod port;

impl_opaquekey_wrapper!(StringId, u32);

impl TableKey for StringId {}
