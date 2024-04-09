mod component;
mod module;
mod port;
mod reference;

pub use component::{PyComponent, PyComponentClass, PyConnectionKind};
pub use module::{json_dumps, json_loads, PyModule_};
pub use port::{PyComponentRefPort, PyPort, PyPortClass, PyPortKind, PyPortPins, PyPortSelection};
pub use reference::PyComponentRef;

use pyo3::prelude::*;

#[pyfunction]
fn smoke_test(py: Python<'_>) -> PyResult<()> {
    use pyo3::types::PyString;

    let name = PyString::new_bound(py, "mod");
    let module = Bound::new(py, PyModule_::new(&name)?)?;

    let module_ref = module.borrow();
    let inner = &module_ref.inner;
    println!("{}", inner.name());

    // let module_copy = Bound::new(py, *module_ref)?;
    // let module_copy_ref = module_copy.borrow();
    // let inner_copy = &module_copy_ref.inner;

    // let mut module_mut_ref = module.borrow_mut();
    // let inner_mut = &mut module_mut_ref.inner;
    // inner_mut.rename("modd");

    Ok(())
}

pub fn register_arch(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = module.py();

    let arch = PyModule::new_bound(py, "arch")?;

    arch.add_class::<PyModule_>()?;
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

    arch.add_function(wrap_pyfunction!(smoke_test, &arch)?)?;

    module.add_submodule(&arch)?;

    Ok(())
}
