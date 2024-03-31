use crate::stringtable::TableKey;

pub use component::{ComponentClass, ComponentId};
pub use module::Module;
pub use port::{PortClass, PortId, PortKind};

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

pub(crate) use impl_dbkey_wrapper;

pub mod component;
pub mod module;
pub mod port;

impl_opaquekey_wrapper!(StringId, u32);

impl TableKey for StringId {}

// #[derive(Deserialize, Eq, Hash, PartialEq)]
// #[serde(untagged)]
// pub enum ItemOrPair<T, U = T, V = T> {
//     #[serde(bound = "T: Deserialize<'de>")]
//     Item(T),
//     #[serde(bound = "U: Deserialize<'de>, V: Deserialize<'de>")]
//     Pair((U, V)),
// }

// impl<T, U, V> ItemOrPair<T, U, V> {
//     pub fn item(item: T) -> Self {
//         Self::Item(item)
//     }

//     pub fn pair(first: U, second: V) -> Self {
//         Self::Pair((first, second))
//     }

//     pub fn get_item(&self) -> Option<&T> {
//         match self {
//             Self::Item(item) => Some(item),
//             Self::Pair((_, _)) => None,
//         }
//     }

//     pub fn get_first(&self) -> Option<&U> {
//         match self {
//             Self::Item(_) => None,
//             Self::Pair((first, _)) => Some(first),
//         }
//     }

//     pub fn get_second(&self) -> Option<&V> {
//         match self {
//             Self::Item(_) => None,
//             Self::Pair((_, second)) => Some(second),
//         }
//     }
// }

// impl<T, V> ItemOrPair<T, T, V> {
//     pub fn item_or_first(&self) -> &T {
//         match self {
//             Self::Item(item) => item,
//             Self::Pair((first, _)) => first,
//         }
//     }
// }

// impl<T, U> ItemOrPair<T, U, T> {
//     pub fn item_or_second(&self) -> &T {
//         match self {
//             Self::Item(item) => item,
//             Self::Pair((_, second)) => second,
//         }
//     }
// }

// impl<T> ItemOrPair<T, T, T> {
//     pub fn transform<F, O>(&self, mut f: F) -> ItemOrPair<O>
//     where
//         F: FnMut(&T) -> O,
//     {
//         match self {
//             Self::Item(item) => ItemOrPair::Item((f)(item)),
//             Self::Pair((first, second)) => ItemOrPair::Pair(((f)(first), (f)(second))),
//         }
//     }
// }
