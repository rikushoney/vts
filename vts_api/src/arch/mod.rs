mod component;
mod module;
mod port;
mod reference;

pub use component::{PyComponent, PyComponentClass, PySignature};
pub use module::PyModule_;
pub use port::{PyComponentRefPort, PyPort, PyPortClass, PyPortKind, PyPortPins};
pub use reference::{PyComponentRef, PyComponentRefSelection};

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

#[derive(Clone, Debug, FromPyObject)]
pub enum IntoSignature<'py> {
    #[pyo3(annotation = "Signature")]
    Signature(Bound<'py, PySignature>),
    #[pyo3(annotation = "ComponentRefPort")]
    PortRef(Bound<'py, PyComponentRefPort>),
    #[pyo3(annotation = "Port")]
    Port(Bound<'py, PyPort>),
}

impl<'py> IntoSignature<'py> {
    pub fn into_signature(self) -> PyResult<Bound<'py, PySignature>> {
        match self {
            Self::Signature(signature) => Ok(signature),
            Self::PortRef(reference) => {
                let py = reference.py();

                Bound::new(
                    py,
                    reference.borrow().__getitem__(py, SliceOrIndex::full(py))?,
                )
            }
            Self::Port(port) => {
                let py = port.py();
                Bound::new(py, port.borrow().__getitem__(py, SliceOrIndex::full(py))?)
            }
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
        PyModule_,
        PyComponent,
        PyComponentClass,
        PyComponentRef,
        PyComponentRefPort,
        PyComponentRefSelection,
        PyPort,
        PyPortClass,
        PyPortKind,
        PyPortPins,
        PySignature,
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
