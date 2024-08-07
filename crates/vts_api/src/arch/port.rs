use pyo3::{prelude::*, types::PyString};

use super::{
    connection::{Connector, IntoSignature},
    SliceOrIndex,
};

use vts_core::arch1::{
    module::PortId,
    port::{PinRange, PortPins},
    PortClass, PortKind,
};

use super::prelude::*;

wrap_enum!(
    PyPortClass (name = "PortClass", help = "port class") => PortClass:
        CLOCK = Clock (alias = "clock" | "clk"),
        LUT_IN = LutIn (alias = "lut_in"),
        LUT_OUT = LutOut (alias = "lut_out"),
        LATCH_IN = LatchIn (alias = "latch_in" | "ff_in"),
        LATCH_OUT = LatchOut (alias = "latch_out" | "ff_out"),
);

wrap_enum!(
    PyPortKind (name = "PortKind", help = "port kind") => PortKind:
        INPUT = Input (alias = "i" | "in" | "input"),
        OUTPUT = Output (alias = "o" | "out" | "output"),
);

#[pyclass(name = "Port")]
#[derive(Clone, Debug)]
pub struct PyPort(Py<PyModule_>, PortId);

macro_rules! borrow_inner {
    ($slf:ident + $py:ident => $port:ident) => {
        let module = $slf.module($py).borrow();
        let inner = module.inner.borrow($py);
        let $port = inner
            .0
            .get_port($slf.id())
            .expect("port should be in module");
    };
}

impl PyPort {
    pub(crate) fn new<'py>(
        module: Borrowed<'_, 'py, PyModule_>,
        port: PortId,
    ) -> PyResult<Bound<'py, Self>> {
        let py = module.py();

        if let Some(port) = module.borrow().ports.get(&port) {
            return Ok(port.bind(py).clone());
        }

        let py_port = Py::new(py, Self(module.as_unbound().clone_ref(py), port))?;
        module.borrow_mut().ports.insert(port, py_port.clone());
        Ok(py_port.bind(py).clone())
    }

    pub(crate) fn id(&self) -> PortId {
        self.1
    }
}

#[pymethods]
impl PyPort {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn parent<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => port);
        PyComponent::new(self.module(py).into(), port.parent().unbind())
    }

    pub fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        borrow_inner!(self + py => port);
        PyString::new_bound(py, port.name())
    }

    pub fn kind(&self, py: Python<'_>) -> PyPortKind {
        borrow_inner!(self + py => port);
        PyPortKind::from(port.kind())
    }

    pub fn n_pins(&self, py: Python<'_>) -> u32 {
        borrow_inner!(self + py => port);
        port.n_pins()
    }

    #[pyo3(name = "class_")]
    pub fn class(&self, py: Python<'_>) -> Option<PyPortClass> {
        borrow_inner!(self + py => port);
        port.class().map(PyPortClass::from)
    }

    #[pyo3(name = "select")]
    pub fn select_py(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PyPortPins> {
        let n_pins = self.n_pins(py);
        let mut range = PinRange::Bound(index.to_range(n_pins)?);
        range.flatten(n_pins);
        borrow_inner!(self + py => port);

        Ok(PyPortPins::new(
            self.module(py).as_borrowed(),
            port.select(range),
        ))
    }

    pub fn __getitem__(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PySignature> {
        let pins = self.select_py(py, index)?;
        let parent = {
            borrow_inner!(self + py => port);
            port.parent().unbind()
        };

        Ok(PySignature::new_component(parent, pins))
    }

    pub fn __setitem__(
        &self,
        py: Python<'_>,
        sink: SliceOrIndex<'_>,
        source: Connector<'_>,
    ) -> PyResult<()> {
        let sink = Bound::new(py, self.__getitem__(py, sink)?)?;

        source.connect(
            self.parent(py)?.as_borrowed(),
            IntoSignature::Signature(sink.unbind()),
        )
    }
}

#[pyclass(name = "PortPins")]
#[derive(Clone, Debug)]
pub struct PyPortPins(Py<PyModule_>, pub(crate) PortPins);

impl PyPortPins {
    pub(crate) fn new(module: Borrowed<'_, '_, PyModule_>, pins: PortPins) -> Self {
        Self(module.to_owned().unbind(), pins)
    }

    pub fn len(&self, py: Python<'_>) -> u32 {
        let module = self.0.borrow(py);
        let inner = module.inner.borrow(py);
        self.1.len(&inner.0)
    }
}
