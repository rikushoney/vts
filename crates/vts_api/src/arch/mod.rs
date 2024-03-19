mod component;
mod module;
mod port;

pub use component::{PyComponent, PyComponentClass};
pub use module::PyModule_ as PyModule;
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

pub(crate) use {map_py_de_err, map_py_ser_err};
