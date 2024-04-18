mod component;
mod connection;
mod module;
mod port;
mod prelude;
mod reference;

pub use prelude::*;

use std::ops::Range;

use pyo3::{
    exceptions::{PyException, PyValueError},
    prelude::*,
    types::{PySlice, PySliceIndices},
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

#[derive(FromPyObject)]
pub enum SliceOrIndex<'py> {
    #[pyo3(annotation = "slice")]
    Slice(Bound<'py, PySlice>),
    #[pyo3(annotation = "int")]
    Index(u32),
}

impl<'py> SliceOrIndex<'py> {
    pub fn full(py: Python<'py>) -> Self {
        Self::Slice(PySlice::full_bound(py))
    }

    fn validate_slice(start: isize, stop: isize, step: isize) -> PyResult<()> {
        if step != 1 {
            return Err(PyValueError::new_err(
                "only port slicing with step size 1 is supported",
            ));
        }

        if start < 0 {
            return Err(PyValueError::new_err("start should be non-negative"));
        }

        if stop < 0 {
            return Err(PyValueError::new_err("stop should be non-negative"));
        }

        if start == stop {
            return Err(PyValueError::new_err("empty slice"));
        }

        if start > stop {
            return Err(PyValueError::new_err("stop should be greater than start"));
        }

        Ok(())
    }

    pub fn to_range(&self, n_pins: u32) -> PyResult<Range<u32>> {
        match self {
            Self::Slice(slice) => {
                let PySliceIndices {
                    start, stop, step, ..
                } = slice.indices(n_pins as i64)?;

                Self::validate_slice(start, stop, step)?;

                Ok(Range {
                    start: start as u32,
                    end: stop as u32,
                })
            }
            Self::Index(index) => Ok(Range {
                start: *index,
                end: *index + 1,
            }),
        }
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
        PyComplete,
        PyComponent,
        PyComponentClass,
        PyComponentRef,
        PyComponentRefPort,
        PyComponentRefSelection,
        PyConcat,
        PyConnectionKind,
        PyDirect,
        PyModule_,
        PyMux,
        PyPort,
        PyPortClass,
        PyPortKind,
        PyPortPins,
        PySignature,
    });

    register_functions!(arch {
        connection::complete,
        connection::concat,
        connection::direct,
        connection::mux,
        module::json_dumps,
        module::json_loads,
        module::toml_dumps,
        module::toml_loads,
        module::yaml_dumps,
        module::yaml_loads,
        smoke_test,
    });

    module.add_submodule(&arch)?;
    Ok(arch)
}
