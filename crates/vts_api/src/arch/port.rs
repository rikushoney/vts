use std::ops::Range;

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
        name: &Bound<'_, PyString>,
        kind: PyPortKind,
        n_pins: Option<usize>,
        class_: Option<PyPortClass>,
    ) -> Self {
        let name = name.clone().unbind();
        let n_pins = n_pins.unwrap_or(1);

        Self {
            name,
            kind,
            n_pins,
            class_,
        }
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let name = PyString::new_bound(py, self.name.bind(py).to_str()?);

        Ok(Self::new(&name, self.kind, Some(self.n_pins), self.class_))
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPortPins {
    #[pyo3(get, set)]
    pub port: Py<PyPort>,
    pub(crate) range: Range<u32>,
}

#[pymethods]
impl PyPortPins {
    #[new]
    pub fn new(port: Bound<'_, PyPort>, start: Option<u32>, end: Option<u32>) -> Self {
        let start = start.unwrap_or(0);
        let end = end.unwrap_or_else(|| {
            let port = port.borrow();
            port.n_pins as u32
        });

        let port = port.unbind();
        let range = start..end;

        Self { port, range }
    }

    #[getter]
    pub fn get_start(&self) -> u32 {
        self.range.start
    }

    #[getter]
    pub fn get_end(&self) -> u32 {
        self.range.end
    }

    #[setter]
    pub fn set_start(&mut self, start: u32) {
        self.range.start = start;
    }

    #[setter]
    pub fn set_end(&mut self, end: u32) {
        self.range.end = end;
    }
}
