use std::ops::Range;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PySlice, PySliceIndices, PyString};
use vts_core::arch::{
    component::ComponentKey,
    port::{PinRange, PortKey, PortPins},
    reference::ComponentRefKey,
    PortClass, PortKind,
};

use super::{PyComponent, PyComponentRef, PyConnectionKind, PyModule_};

wrap_enum!(
    PyPortClass as "port class" => PortClass:
        CLOCK = Clock ("clock" | "clk"),
        LUT_IN = LutIn ("lut_in"),
        LUT_OUT = LutOut ("lut_out"),
        LATCH_IN = LatchIn ("latch_in" | "ff_in"),
        LATCH_OUT = LatchOut ("latch_out" | "ff_out"),
);

wrap_enum!(
    PyPortKind as "port kind" => PortKind:
        INPUT = Input ("i" | "in" | "input"),
        OUTPUT = Output ("o" | "out" | "output"),
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPort(Py<PyModule_>, PortKey);

macro_rules! get_port {
    ($slf:ident + $py:ident => $port:ident) => {
        let module = $slf.module($py).borrow();
        let $port = module
            .inner
            .get_port($slf.key())
            .expect("port should be in module");
    };
}

impl PyPort {
    pub(crate) fn new<'py>(
        py: Python<'py>,
        module: &Bound<'py, PyModule_>,
        port: PortKey,
    ) -> PyResult<Bound<'py, Self>> {
        if let Some(port) = module.borrow().ports.get(&port) {
            return Ok(port.bind(py).clone());
        }

        let py_port = Py::new(py, Self(module.clone().unbind(), port))?;
        module.borrow_mut().ports.insert(port, py_port.clone());
        Ok(py_port.bind(py).clone())
    }

    pub(crate) fn key(&self) -> PortKey {
        self.1
    }
}

pub enum SliceOrIndex<'py> {
    Slice(Bound<'py, PySlice>),
    Index(u32),
}

impl<'py> SliceOrIndex<'py> {
    pub fn full(py: Python<'py>) -> Self {
        Self::Slice(PySlice::full_bound(py))
    }

    pub fn to_range(&self, n_pins: u32) -> PyResult<Range<u32>> {
        match self {
            SliceOrIndex::Slice(slice) => {
                let PySliceIndices {
                    start, stop, step, ..
                } = slice.indices(n_pins as i64)?;

                if step != 1 {
                    return Err(PyValueError::new_err(
                        "only port selection with step size 1 is supported",
                    ));
                }

                if start < 0 {
                    return Err(PyValueError::new_err("start index should be non-negative"));
                }

                if stop < 0 {
                    return Err(PyValueError::new_err("stop index should be non-negative"));
                }

                if start == stop {
                    return Err(PyValueError::new_err("empty selection"));
                }

                if start > stop {
                    return Err(PyValueError::new_err(
                        "stop index should be greater than start",
                    ));
                }

                Ok(Range {
                    start: start as u32,
                    end: stop as u32,
                })
            }
            SliceOrIndex::Index(index) => Ok(Range {
                start: *index,
                end: *index + 1,
            }),
        }
    }
}

impl<'py> FromPyObject<'py> for SliceOrIndex<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(slice) = ob.downcast::<PySlice>() {
            Ok(SliceOrIndex::Slice(slice.clone()))
        } else if let Ok(index) = ob.extract::<u32>() {
            Ok(SliceOrIndex::Index(index))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                r#"expected slice or int, not "{error_ty}""#
            )))
        }
    }
}

#[pymethods]
impl PyPort {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn parent<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        get_port!(self + py => port);
        PyComponent::new(py, self.module(py), port.parent().key())
    }

    pub fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        get_port!(self + py => port);
        PyString::new_bound(py, port.name())
    }

    pub fn kind(&self, py: Python<'_>) -> PyPortKind {
        get_port!(self + py => port);
        PyPortKind::from(port.kind())
    }

    pub fn n_pins(&self, py: Python<'_>) -> u32 {
        get_port!(self + py => port);
        port.n_pins()
    }

    #[pyo3(name = "class_")]
    pub fn class(&self, py: Python<'_>) -> Option<PyPortClass> {
        get_port!(self + py => port);
        port.class().map(PyPortClass::from)
    }

    fn select(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PyPortPins> {
        let mut range = PinRange::Bound(index.to_range(self.n_pins(py))?);
        range.flatten(self.n_pins(py));
        get_port!(self + py => port);
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

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPortPins(pub(crate) PortPins);

impl PyPortPins {
    pub(crate) fn new(pins: PortPins) -> Self {
        Self(pins)
    }
}

#[derive(Clone, Debug)]
pub enum ComponentOrRef {
    Component(ComponentKey),
    Ref(ComponentRefKey),
}

impl<'py> FromPyObject<'py> for ComponentOrRef {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(component) = ob.downcast::<PyComponent>() {
            let component = component.borrow();
            Ok(ComponentOrRef::Component(component.key()))
        } else if let Ok(reference) = ob.downcast::<PyComponentRef>() {
            let reference = reference.borrow();
            Ok(ComponentOrRef::Ref(reference.key()))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                r#"expected component or reference, not "{error_ty}""#
            )))
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPortSelection(pub(crate) ComponentOrRef, pub(crate) PyPortPins);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyComponentRefPort(pub(crate) Py<PyComponentRef>, pub(crate) Py<PyPort>);

pub enum PortSelectionOrRef<'py> {
    Selection(Bound<'py, PyPortSelection>),
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

impl<'py> FromPyObject<'py> for PortSelectionOrRef<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(selection) = ob.downcast::<PyPortSelection>() {
            Ok(PortSelectionOrRef::Selection(selection.clone()))
        } else if let Ok(reference) = ob.downcast::<PyComponentRefPort>() {
            Ok(PortSelectionOrRef::Ref(reference.clone()))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                r#"expected port selection or reference, not "{error_ty}""#
            )))
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
                py,
                &source,
                &sink.get_selection()?,
                Some(PyConnectionKind::DIRECT),
            )?;

        Ok(())
    }
}
