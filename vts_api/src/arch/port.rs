use std::ops::Range;

use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PySlice, PySliceIndices, PyString},
};
use vts_core::arch::{
    component::ComponentKey,
    port::{PinRange, PortKey, PortPins},
    reference::ComponentRefKey,
    PortClass, PortKind,
};

use super::{PyComponent, PyComponentRef, PyConnectionKind, PyModule_};

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
pub struct PyPort(Py<PyModule_>, PortKey);

macro_rules! borrow_inner {
    ($slf:ident + $py:ident => $port:ident) => {
        let module = $slf.module($py).borrow();
        let inner = module.inner.borrow($py);
        let $port = inner
            .0
            .get_port($slf.key())
            .expect("port should be in module");
    };
}

impl PyPort {
    pub(crate) fn new<'py>(
        module: Borrowed<'_, 'py, PyModule_>,
        port: PortKey,
    ) -> PyResult<Bound<'py, Self>> {
        let py = module.py();

        if let Some(port) = module.borrow().ports.get(&port) {
            return Ok(port.bind(py).clone());
        }

        let py_port = Py::new(py, Self(module.as_unbound().clone_ref(py), port))?;
        module.borrow_mut().ports.insert(port, py_port.clone());
        Ok(py_port.bind(py).clone())
    }

    pub(crate) fn key(&self) -> PortKey {
        self.1
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

#[pymethods]
impl PyPort {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn parent<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => port);
        PyComponent::new(self.module(py).into(), port.parent().key())
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

    fn select(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PyPortPins> {
        let mut range = PinRange::Bound(index.to_range(self.n_pins(py))?);
        range.flatten(self.n_pins(py));
        borrow_inner!(self + py => port);
        Ok(PyPortPins::new(port.select(range)))
    }

    pub fn __getitem__(
        &self,
        py: Python<'_>,
        index: SliceOrIndex<'_>,
    ) -> PyResult<PyPortSelection> {
        let pins = self.select(py, index)?;
        let parent = self.parent(py)?.borrow();

        Ok(PyPortSelection(
            ComponentOrRef::Component(parent.key()),
            pins,
        ))
    }
}

#[pyclass(name = "PortPins")]
#[derive(Clone, Debug)]
pub struct PyPortPins(pub(crate) PortPins);

impl PyPortPins {
    pub(crate) fn new(pins: PortPins) -> Self {
        Self(pins)
    }
}

fn extract_component_key(ob: &Bound<'_, PyAny>) -> PyResult<ComponentKey> {
    ob.downcast::<PyComponent>()
        .map(|component| component.borrow().key())
        .map_err(PyErr::from)
}

fn extract_reference_key(ob: &Bound<'_, PyAny>) -> PyResult<ComponentRefKey> {
    ob.downcast::<PyComponentRef>()
        .map(|reference| reference.borrow().key())
        .map_err(PyErr::from)
}

#[derive(Clone, Debug, FromPyObject)]
pub enum ComponentOrRef {
    #[pyo3(annotation = "Component")]
    Component(#[pyo3(from_py_with = "extract_component_key")] ComponentKey),
    #[pyo3(annotation = "ComponentRef")]
    Ref(#[pyo3(from_py_with = "extract_reference_key")] ComponentRefKey),
}

#[pyclass(name = "PortSelection")]
#[derive(Clone, Debug)]
pub struct PyPortSelection(pub(crate) ComponentOrRef, pub(crate) PyPortPins);

#[pyclass(name = "ComponentRefPort")]
#[derive(Clone, Debug)]
pub struct PyComponentRefPort(pub(crate) Py<PyComponentRef>, pub(crate) Py<PyPort>);

#[derive(FromPyObject)]
pub enum PortSelectionOrRef<'py> {
    #[pyo3(annotation = "PortSelection")]
    Selection(Bound<'py, PyPortSelection>),
    #[pyo3(annotation = "ComponentRefPort")]
    Ref(Bound<'py, PyComponentRefPort>),
}

impl<'py> PortSelectionOrRef<'py> {
    pub fn get_selection(&self) -> PyResult<Bound<'py, PyPortSelection>> {
        match self {
            PortSelectionOrRef::Selection(selection) => Ok(selection.clone()),
            PortSelectionOrRef::Ref(reference) => {
                let py = reference.py();
                let reference = reference.borrow();
                let port = reference.1.bind(py).borrow();
                let reference = ComponentOrRef::Ref(reference.0.borrow(py).key());
                let selection = port.select(py, SliceOrIndex::full(py))?;
                Bound::new(py, PyPortSelection(reference, selection))
            }
        }
    }
}

#[pymethods]
impl PyComponentRefPort {
    pub fn __getitem__(
        &self,
        py: Python<'_>,
        index: SliceOrIndex<'_>,
    ) -> PyResult<PyPortSelection> {
        let port = self.1.bind(py).borrow();
        let pins = port.select(py, index)?;
        let reference = self.0.bind(py).borrow();

        Ok(PyPortSelection(ComponentOrRef::Ref(reference.key()), pins))
    }

    pub fn __setitem__(
        &self,
        py: Python<'_>,
        index: SliceOrIndex<'_>,
        sink: PortSelectionOrRef<'_>,
    ) -> PyResult<()> {
        let source = Bound::new(py, self.__getitem__(py, index)?)?;

        self.0
            .bind(py)
            .borrow()
            .parent(py)?
            .borrow_mut()
            .add_connection(
                &source,
                &sink.get_selection()?,
                Some(PyConnectionKind::DIRECT),
            )?;

        Ok(())
    }
}
