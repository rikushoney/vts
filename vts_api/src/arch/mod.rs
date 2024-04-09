mod component;
mod module;
mod port;
mod reference;

pub use component::{PyComponent, PyComponentClass};
pub use module::{json_dumps, json_loads, PyModule};
pub use port::{PyComponentRefPort, PyPort, PyPortClass, PyPortKind, PyPortPins, PyPortSelection};
pub use reference::PyComponentRef;

use pyo3::prelude::*;

pub fn register_arch(module: &Bound<'_, pyo3::prelude::PyModule>) -> PyResult<()> {
    let py = module.py();

    let arch = PyModule::new_bound(py, "arch")?;

    arch.add_class::<PyModule>()?;
    arch.add_class::<PyComponent>()?;
    arch.add_class::<PyComponentClass>()?;
    arch.add_class::<PyComponentRef>()?;
    arch.add_class::<PyComponentRefPort>()?;
    arch.add_class::<PyPort>()?;
    arch.add_class::<PyPortClass>()?;
    arch.add_class::<PyPortKind>()?;
    arch.add_class::<PyPortPins>()?;
    arch.add_class::<PyPortSelection>()?;

    arch.add_function(wrap_pyfunction!(json_dumps, &arch)?)?;
    arch.add_function(wrap_pyfunction!(json_loads, &arch)?)?;

    module.add_submodule(&arch)?;

    Ok(())
}
