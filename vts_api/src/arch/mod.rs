mod component;
mod module;
mod port;

pub use component::{PyComponent, PyComponentClass, PyComponentRef, PyConnection};
pub use module::{json_dumps, json_loads, PyModule_ as PyModule};
pub use port::{PyPort, PyPortClass, PyPortKind, PyPortPins};

pub(super) mod impl_ {
    use pyo3::prelude::*;

    pub fn register_arch(module: &Bound<'_, pyo3::prelude::PyModule>) -> PyResult<()> {
        let py = module.py();

        let arch = PyModule::new_bound(py, "arch")?;

        arch.add_class::<super::PyModule>()?;
        arch.add_class::<super::PyComponent>()?;
        arch.add_class::<super::PyComponentRef>()?;
        arch.add_class::<super::PyConnection>()?;
        arch.add_class::<super::PyComponentClass>()?;
        arch.add_class::<super::PyPort>()?;
        arch.add_class::<super::PyPortClass>()?;
        arch.add_class::<super::PyPortKind>()?;
        arch.add_class::<super::PyPortPins>()?;

        arch.add_function(wrap_pyfunction!(super::json_dumps, &arch)?)?;
        arch.add_function(wrap_pyfunction!(super::json_loads, &arch)?)?;

        module.add_submodule(&arch)?;

        Ok(())
    }
}

pub(super) use impl_::register_arch;
