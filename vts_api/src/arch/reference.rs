use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::PyString;

use vts_core::arch::reference::ComponentRefKey;

use super::port::SliceOrIndex;
use super::{
    PyComponent, PyComponentRefPort, PyConnectionKind, PyModule_, PyPort, PyPortSelection,
};

#[pyclass]
pub struct PyComponentRef(Py<PyModule_>, ComponentRefKey);

impl PyComponentRef {
    pub(crate) fn new<'py>(
        py: Python<'py>,
        module: Borrowed<'_, 'py, PyModule_>,
        reference: ComponentRefKey,
    ) -> PyResult<Bound<'py, Self>> {
        if let Some(reference) = module.borrow().references.get(&reference) {
            return Ok(reference.bind(py).clone());
        }

        let py_reference = Py::new(py, Self(module.as_unbound().clone_ref(py), reference))?;

        module
            .borrow_mut()
            .references
            .insert(reference, py_reference.clone());

        Ok(py_reference.bind(py).clone())
    }

    pub(crate) fn key(&self) -> ComponentRefKey {
        self.1
    }
}

macro_rules! borrow_inner {
    ($slf:ident + $py:ident => $ref:ident) => {
        let module = $slf.module($py).borrow();
        let $ref = module
            .inner
            .get_reference($slf.key())
            .expect("reference should be in module");
    };
}

#[pymethods]
impl PyComponentRef {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn component<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => reference);
        PyComponent::new(py, self.module(py).into(), reference.component().key())
    }

    pub fn parent<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => reference);
        PyComponent::new(py, self.module(py).into(), reference.parent().key())
    }

    pub fn alias<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyString>> {
        borrow_inner!(self + py => reference);

        reference
            .alias()
            .map(|alias| PyString::new_bound(py, alias))
    }

    pub fn alias_or_name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        borrow_inner!(self + py => reference);
        PyString::new_bound(py, reference.alias_or_name())
    }

    pub fn n_instances(&self, py: Python<'_>) -> usize {
        borrow_inner!(self + py => reference);
        reference.n_instances()
    }

    pub fn __getattr__(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        port: &Bound<'_, PyString>,
    ) -> PyResult<PyComponentRefPort> {
        let reference = slf.borrow();
        borrow_inner!(reference + py => reference);
        let port = port.to_str()?;

        if let Some(port) = reference.component().find_port(port) {
            let port = PyPort::new(py, slf.borrow().module(py).as_borrowed(), port.key())?;
            Ok(PyComponentRefPort(slf.clone().unbind(), port.unbind()))
        } else {
            let component = reference.component().name();
            Err(PyAttributeError::new_err(format!(
                r#"undefined port "{port}" referenced in "{component}""#
            )))
        }
    }

    pub fn __setattr__(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        source: &Bound<'_, PyString>,
        sink: &Bound<'_, PyPortSelection>,
    ) -> PyResult<()> {
        let port = Self::__getattr__(slf, py, source)?;

        let source = {
            let source = port.__getitem__(py, SliceOrIndex::full(py))?;
            Bound::new(py, source)?
        };

        let reference = slf.borrow();
        let parent = reference.parent(py)?;

        parent
            .borrow_mut()
            .add_connection(py, &source, sink, Some(PyConnectionKind::DIRECT))
    }
}
