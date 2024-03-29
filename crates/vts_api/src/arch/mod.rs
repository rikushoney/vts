mod component;
mod module;
mod port;

pub use component::{PyComponent, PyComponentClass};
pub use module::{json_dumps, json_loads, PyModule_ as PyModule};
pub use port::{PyPort, PyPortClass, PyPortKind};

macro_rules! map_py_de_err {
    ($expr:expr) => {
        ($expr).map_err(|err| de::Error::custom(err.to_string()))
    };
}

macro_rules! map_py_ser_err {
    ($expr:expr) => {
        ($expr).map_err(|err| ser::Error::custom(err.to_string()))
    };
}

macro_rules! iter_dict_items {
    (for ( $key:ident : $key_ty:ty, $val:ident : $val_ty:ty ) in $dict:expr => $runner:expr) => {
        for ($key, $val) in $dict.iter() {
            let $key = $key.downcast::<$key_ty>()?;
            let $val = $val.downcast::<$val_ty>()?;
            $runner
        }
    };
}

macro_rules! iter_mapping_items {
    (for ( $key:ident : $key_ty:ty, $val:ident : $val_ty:ty) in $mapping:expr => $runner:expr) => {
        let items = $mapping.items()?.to_list()?;
        for item in items.iter() {
            let item = item.downcast::<pyo3::types::PyTuple>()?;
            let $key = item.get_borrowed_item(0)?;
            let $key = $key.as_any().downcast::<$key_ty>()?;
            let $val = item.get_borrowed_item(1)?;
            let $val = $val.as_any().downcast::<$val_ty>()?;
            $runner
        }
    };
}

pub(crate) use {iter_dict_items, iter_mapping_items, map_py_de_err, map_py_ser_err};
