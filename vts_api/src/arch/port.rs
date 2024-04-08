use pyo3::prelude::*;
use pyo3::types::PyString;
use vts_core::arch::{port::PortKey, PortClass, PortKind};

use super::module::PyModule_;

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
pub struct PyPort(Py<PyModule_>, PortKey);

impl PyPort {
    pub(crate) fn new(module: &Bound<'_, PyModule_>, port: PortKey) -> Self {
        let module = module.clone().unbind();
        PyPort(module, port)
    }

    pub(crate) fn key(&self) -> PortKey {
        self.1
    }
}

#[pymethods]
impl PyPort {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        let module = self.module(py).borrow();
        let port = module
            .0
            .get_port(self.key())
            .expect("port should be in module");

        PyString::new_bound(py, port.name())
    }

    pub fn kind(&self, py: Python<'_>) -> PyPortKind {
        let module = self.module(py).borrow();
        let port = module
            .0
            .get_port(self.key())
            .expect("port should be in module");

        PyPortKind::from(port.kind())
    }

    pub fn n_pins(&self, py: Python<'_>) -> usize {
        let module = self.module(py).borrow();
        let port = module
            .0
            .get_port(self.key())
            .expect("port should be in module");

        port.n_pins()
    }

    #[pyo3(name = "class_")]
    pub fn class(&self, py: Python<'_>) -> Option<PyPortClass> {
        let module = self.module(py).borrow();
        let port = module
            .0
            .get_port(self.key())
            .expect("port should be in module");

        port.class().map(PyPortClass::from)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPortPins {}

#[pymethods]
impl PyPortPins {}
