pub mod de;
pub mod ser;

use pyo3::prelude::*;
use pyo3::types::PyString;
use vts_core::arch::{PortClass, PortKind};

wrap_enum!(PyPortClass => PortClass:
    CLOCK = Clock,
    LUT_IN = LutIn,
    LUT_OUT = LutOut,
    LATCH_IN = LatchIn,
    LATCH_OUT = LatchOut,
);

wrap_enum!(PyPortKind => PortKind:
    INPUT = Input,
    OUTPUT = Output,
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPort {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub kind: PyPortKind,
    #[pyo3(get, set)]
    pub n_pins: usize,
    #[pyo3(get, set)]
    pub class_: Option<PyPortClass>,
}

#[pymethods]
impl PyPort {
    #[new]
    pub fn new(
        _py: Python<'_>,
        name: Py<PyString>,
        kind: PyPortKind,
        n_pins: Option<usize>,
        class_: Option<PyPortClass>,
    ) -> PyResult<Self> {
        Ok(Self {
            name,
            kind,
            n_pins: n_pins.unwrap_or(1),
            class_,
        })
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let name = PyString::new(py, self.name.as_ref(py).to_str()?);
        Self::new(
            py,
            name.into_py(py),
            self.kind,
            Some(self.n_pins),
            self.class_,
        )
    }
}
