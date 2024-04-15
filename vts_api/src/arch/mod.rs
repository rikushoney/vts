mod component;
mod module;
mod port;
mod reference;

pub use component::{PyComponent, PyComponentClass, PyConnectionKind};
pub use module::PyModule_;
pub use port::{PyComponentRefPort, PyPort, PyPortClass, PyPortKind, PyPortPins, PyPortSelection};
pub use reference::PyComponentRef;

use pyo3::{
    exceptions::{PyException, PyValueError},
    prelude::*,
};
use vts_core::arch::{checker, Error};

#[pyfunction]
fn smoke_test(py: Python<'_>) -> PyResult<()> {
    use pyo3::types::PyString;

    let name = PyString::new_bound(py, "mod");
    let module = Bound::new(py, PyModule_::new(&name)?)?;

    let module_ref = module.borrow();
    let inner = &module_ref.inner;
    println!("{}", inner.borrow(py).0.name());

    Ok(())
}

struct PyError(Error);

impl From<Error> for PyError {
    fn from(err: Error) -> Self {
        Self(err)
    }
}

impl From<PyError> for PyErr {
    fn from(PyError(err): PyError) -> Self {
        match err {
            Error::Linker(err) => PyValueError::new_err(err.to_string()),
            Error::Checker(err) => PyValueError::new_err(err.to_string()),
            Error::Generic(err) => PyException::new_err(err.to_string()),
        }
    }
}

struct PyCheckerError(checker::Error);

impl From<checker::Error> for PyCheckerError {
    fn from(err: checker::Error) -> Self {
        Self(err)
    }
}

impl From<PyCheckerError> for PyErr {
    fn from(PyCheckerError(err): PyCheckerError) -> Self {
        PyValueError::new_err(err.to_string())
    }
}

macro_rules! register_classes {
    ($arch:ident { $($class:path),* $(,)? }) => {
        $(
            $arch.add_class::<$class>()?;
        )*
    }
}

macro_rules! register_functions {
    ($arch:ident { $($function:path),* $(,)? }) => {
        $(
            $arch.add_function(wrap_pyfunction!($function, &$arch)?)?;
        )*
    }
}

pub fn register_arch<'py>(module: &Bound<'py, PyModule>) -> PyResult<Bound<'py, PyModule>> {
    let py = module.py();
    let arch = PyModule::new_bound(py, "arch")?;

    register_classes!(arch {
        PyModule_,
        PyComponent,
        PyComponentClass,
        PyComponentRef,
        PyComponentRefPort,
        PyPort,
        PyPortClass,
        PyPortKind,
        PyPortPins,
        PyPortSelection
    });

    register_functions!(arch {
        module::json_dumps,
        module::json_loads,
        module::yaml_dumps,
        module::yaml_loads,
        module::toml_dumps,
        module::toml_loads,
        smoke_test,
    });

    module.add_submodule(&arch)?;
    Ok(arch)
}
