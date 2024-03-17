use crate::stringtable::TableKey;

pub use component::{Component, ComponentClass};
pub use module::Module;
pub use port::{Port, PortClass, PortKind};

macro_rules! impl_opaquekey_wrapper {
    ($name:ident, $base:path) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name($base);

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

        impl crate::database::DbKey for $name {}
    };
}

// macro_rules! assert_ptr_eq {
//     ($left:expr, $right:expr) => {
//         assert!(std::ptr::eq($left as *const _, $right as *const _))
//     };
// }

pub(crate) use impl_dbkey_wrapper;

pub mod component;
pub mod module;
pub mod port;

impl_opaquekey_wrapper!(StringId, u32);

impl TableKey for StringId {}
